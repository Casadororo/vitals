//! macOS readings from the kernel: CPU time per core, memory split the way
//! Activity Monitor splits it, memory pressure, swap, and task and thread
//! counts, none of which need special rights.

use std::ffi::CString;
use std::mem;
use std::time::Instant;

use super::{Ticks, rate};
use crate::model::{Memory, Part, PressureLevel, Swap, Tasks};

/// `processor_set_load_info` from <mach/processor_info.h>.
#[repr(C)]
#[derive(Default)]
struct ProcessorSetLoadInfo {
    task_count: libc::c_int,
    thread_count: libc::c_int,
    load_average: libc::integer_t,
    mach_factor: libc::integer_t,
}

const PROCESSOR_SET_LOAD_INFO: libc::c_int = 4;

unsafe extern "C" {
    fn processor_set_default(
        host: libc::mach_port_t,
        set: *mut libc::mach_port_t,
    ) -> libc::kern_return_t;
    fn processor_set_statistics(
        set: libc::mach_port_t,
        flavor: libc::c_int,
        info: *mut libc::integer_t,
        count: *mut libc::mach_msg_type_number_t,
    ) -> libc::kern_return_t;
    fn mach_port_deallocate(
        task: libc::mach_port_t,
        name: libc::mach_port_t,
    ) -> libc::kern_return_t;
}

/// This process's task port.
fn task_self() -> libc::mach_port_t {
    // SAFETY: reads the port the system set up when the process started.
    #[allow(deprecated)]
    unsafe {
        libc::mach_task_self()
    }
}

pub struct Mac {
    host: libc::mach_port_t,
    page_size: u64,
    /// Swap page counters and when they were read.
    swap_pages: Option<((u64, u64), Instant)>,
}

impl Mac {
    pub fn new() -> Self {
        // SAFETY: both only read; the host port is given back in `drop`.
        #[allow(deprecated)]
        let host = unsafe { libc::mach_host_self() };
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        Self {
            host,
            page_size: u64::try_from(page_size).unwrap_or(16_384),
            swap_pages: None,
        }
    }

    /// Time counters of each core: user, system, idle and nice.
    pub fn ticks(&self) -> Option<Vec<Ticks>> {
        let mut cores: libc::natural_t = 0;
        let mut info: libc::processor_info_array_t = std::ptr::null_mut();
        let mut info_count: libc::mach_msg_type_number_t = 0;
        // SAFETY: the kernel allocates `info` with one load per core; it is
        // copied out and handed back before returning.
        unsafe {
            let result = libc::host_processor_info(
                self.host,
                libc::PROCESSOR_CPU_LOAD_INFO,
                &mut cores,
                &mut info,
                &mut info_count,
            );
            if result != libc::KERN_SUCCESS || info.is_null() {
                return None;
            }
            let loads = std::slice::from_raw_parts(
                info.cast::<libc::processor_cpu_load_info>(),
                cores as usize,
            );
            let ticks = loads
                .iter()
                .map(|load| {
                    let tick = |state: libc::c_int| u64::from(load.cpu_ticks[state as usize]);
                    Ticks {
                        user: tick(libc::CPU_STATE_USER),
                        system: tick(libc::CPU_STATE_SYSTEM),
                        idle: tick(libc::CPU_STATE_IDLE),
                        nice: tick(libc::CPU_STATE_NICE),
                        ..Ticks::default()
                    }
                })
                .collect();
            libc::vm_deallocate(
                task_self(),
                info as libc::vm_address_t,
                info_count as libc::vm_size_t * mem::size_of::<libc::integer_t>(),
            );
            Some(ticks)
        }
    }

    /// Memory as Activity Monitor splits it (used is app, wired and
    /// compressed memory; cached files are not used), the pressure, and swap.
    pub fn memory(&mut self, now: Instant) -> Option<(Memory, Swap)> {
        // SAFETY: a plain C struct that the kernel fills in.
        let mut stats: libc::vm_statistics64 = unsafe { mem::zeroed() };
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let result = unsafe {
            libc::host_statistics64(
                self.host,
                libc::HOST_VM_INFO64,
                (&raw mut stats).cast(),
                &mut count,
            )
        };
        if result != libc::KERN_SUCCESS {
            return None;
        }
        let total = sysctl_u64("hw.memsize")?;
        let bytes = |pages: u64| pages * self.page_size;
        let purgeable = u64::from(stats.purgeable_count);
        let app = bytes(u64::from(stats.internal_page_count).saturating_sub(purgeable));
        let wired = bytes(u64::from(stats.wire_count));
        let compressed = bytes(u64::from(stats.compressor_page_count));
        let cached = bytes(u64::from(stats.external_page_count) + purgeable);
        let pressure = sysctl_u64("kern.memorystatus_vm_pressure_level").map(|level| match level {
            4 => PressureLevel::Critical,
            2 => PressureLevel::Warning,
            _ => PressureLevel::Normal,
        });
        let memory = Memory {
            total,
            used: (app + wired + compressed).min(total),
            parts: vec![
                (Part::App, app),
                (Part::Wired, wired),
                (Part::Compressed, compressed),
                (Part::Cached, cached),
            ],
            available: None,
            pressure,
            stall: None,
        };

        let pages = (stats.swapins, stats.swapouts);
        let activity = self.swap_pages.map(|((before_in, before_out), then)| {
            let seconds = now.duration_since(then).as_secs_f64();
            (
                rate(bytes(pages.0.saturating_sub(before_in)), seconds),
                rate(bytes(pages.1.saturating_sub(before_out)), seconds),
            )
        });
        self.swap_pages = Some((pages, now));
        let (swap_total, swap_used) = swap_usage().unwrap_or((0, 0));
        let swap = Swap {
            total: swap_total,
            used: swap_used,
            activity,
        };
        Some((memory, swap))
    }

    /// Tasks and threads of the whole system, as `top` counts them.
    pub fn tasks(&self) -> Option<Tasks> {
        let mut set: libc::mach_port_t = 0;
        let mut info = ProcessorSetLoadInfo::default();
        let mut count = (mem::size_of::<ProcessorSetLoadInfo>() / mem::size_of::<libc::integer_t>())
            as libc::mach_msg_type_number_t;
        // SAFETY: `info` has room for `count` integers; the processor set's
        // name port is given back right away.
        unsafe {
            if processor_set_default(self.host, &mut set) != libc::KERN_SUCCESS {
                return None;
            }
            let result = processor_set_statistics(
                set,
                PROCESSOR_SET_LOAD_INFO,
                (&raw mut info).cast(),
                &mut count,
            );
            mach_port_deallocate(task_self(), set);
            if result != libc::KERN_SUCCESS {
                return None;
            }
        }
        Some(Tasks {
            processes: u64::try_from(info.task_count).ok()?,
            threads: u64::try_from(info.thread_count).ok(),
            running: None,
        })
    }
}

impl Drop for Mac {
    fn drop(&mut self) {
        // SAFETY: gives back the send right taken in `new`.
        unsafe {
            mach_port_deallocate(task_self(), self.host);
        }
    }
}

/// Performance and efficiency cores of Apple silicon, fastest first.
pub fn core_kinds() -> Vec<(String, usize)> {
    let levels = sysctl_u64("hw.nperflevels").unwrap_or(0);
    if levels < 2 {
        return Vec::new();
    }
    (0..levels)
        .filter_map(|level| {
            let cores = sysctl_u64(&format!("hw.perflevel{level}.logicalcpu"))?;
            let name = sysctl_string(&format!("hw.perflevel{level}.name")).unwrap_or_default();
            let short = match name.as_str() {
                "Performance" => "P".to_owned(),
                "Efficiency" => "E".to_owned(),
                other => other.chars().take(1).collect(),
            };
            Some((short, usize::try_from(cores).ok()?))
        })
        .collect()
}

/// Total and used swap, in bytes.
fn swap_usage() -> Option<(u64, u64)> {
    // SAFETY: a plain C struct that the kernel fills in.
    let mut usage: libc::xsw_usage = unsafe { mem::zeroed() };
    let mut size = mem::size_of::<libc::xsw_usage>();
    let result = unsafe {
        libc::sysctlbyname(
            c"vm.swapusage".as_ptr(),
            (&raw mut usage).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (result == 0).then_some((usage.xsu_total, usage.xsu_used))
}

fn sysctl(name: &str, buffer: &mut [u8]) -> Option<usize> {
    let name = CString::new(name).ok()?;
    let mut size = buffer.len();
    // SAFETY: the kernel writes at most `size` bytes into `buffer`.
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (result == 0).then_some(size)
}

fn sysctl_u64(name: &str) -> Option<u64> {
    let mut buffer = [0u8; 8];
    match sysctl(name, &mut buffer)? {
        4 => Some(u64::from(u32::from_ne_bytes(buffer[..4].try_into().ok()?))),
        8 => Some(u64::from_ne_bytes(buffer)),
        _ => None,
    }
}

fn sysctl_string(name: &str) -> Option<String> {
    let mut buffer = [0u8; 64];
    let size = sysctl(name, &mut buffer)?;
    let text = &buffer[..size];
    let end = text
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(text.len());
    String::from_utf8(text[..end].to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_core_and_the_memory() {
        let mut mac = Mac::new();
        let ticks = mac.ticks().expect("CPU ticks");
        assert_eq!(
            ticks.len(),
            std::thread::available_parallelism().unwrap().get()
        );
        let (memory, swap) = mac.memory(Instant::now()).expect("memory");
        assert!(memory.total > 0 && memory.used <= memory.total);
        assert!(memory.pressure.is_some());
        assert!(swap.used <= swap.total);
        let tasks = mac.tasks().expect("tasks");
        assert!(tasks.processes > 1 && tasks.threads.unwrap() >= tasks.processes);
    }
}
