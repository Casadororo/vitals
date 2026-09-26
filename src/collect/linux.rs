//! Linux readings straight from /proc: CPU time split like htop's, memory,
//! swap activity, threads and pressure stall.

use std::fs;
use std::time::Instant;

use super::{Ticks, procfs, rate};
use crate::model::{Memory, Swap, Tasks};

pub struct Linux {
    page_size: u64,
    /// Swap page counters and when they were read.
    swap_pages: Option<((u64, u64), Instant)>,
}

impl Linux {
    pub fn new() -> Self {
        // SAFETY: sysconf only reads a system setting.
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        Self {
            page_size: u64::try_from(page_size).unwrap_or(4096),
            swap_pages: None,
        }
    }

    pub fn ticks(&self) -> Option<Vec<Ticks>> {
        let cores = procfs::stat_cores(&fs::read_to_string("/proc/stat").ok()?);
        (!cores.is_empty()).then_some(cores)
    }

    pub fn memory(&mut self, now: Instant) -> Option<(Memory, Swap)> {
        let text = fs::read_to_string("/proc/meminfo").ok()?;
        let (mut memory, mut swap) = procfs::memory(&procfs::meminfo(&text))?;
        memory.stall = stall("memory");
        if let Some(pages) = fs::read_to_string("/proc/vmstat")
            .ok()
            .and_then(|text| procfs::vmstat_swap(&text))
        {
            if let Some(((before_in, before_out), then)) = self.swap_pages {
                let seconds = now.duration_since(then).as_secs_f64();
                swap.activity = Some((
                    rate(pages.0.saturating_sub(before_in) * self.page_size, seconds),
                    rate(pages.1.saturating_sub(before_out) * self.page_size, seconds),
                ));
            }
            self.swap_pages = Some((pages, now));
        }
        Some((memory, swap))
    }

    pub fn tasks(&self) -> Option<Tasks> {
        let (_, mut tasks) = procfs::loadavg(&fs::read_to_string("/proc/loadavg").ok()?)?;
        tasks.processes = fs::read_dir("/proc")
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.bytes().all(|byte| byte.is_ascii_digit()))
            })
            .count() as u64;
        Some(tasks)
    }
}

/// Pressure stall of "cpu", "memory" or "io", on kernels that report it.
pub fn stall(resource: &str) -> Option<f64> {
    procfs::pressure(&fs::read_to_string(format!("/proc/pressure/{resource}")).ok()?)
}
