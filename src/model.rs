//! One reading of the computer: what the collector measures and the screen shows.

/// Share of CPU time since the previous reading, each part from 0 to 1, all of
/// them adding up to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CpuTimes {
    pub user: f64,
    /// User time of programs running at a lower priority.
    pub nice: f64,
    /// Kernel time, interrupts included.
    pub system: f64,
    /// Time a virtual machine waited for its host to run it (Linux).
    pub steal: f64,
    /// Idle time while waiting for the disk (Linux). The CPU was free, so it
    /// does not count as busy.
    pub iowait: f64,
    pub idle: f64,
}

impl CpuTimes {
    /// When only the busy share is known, it all goes to `user`.
    pub fn from_busy(busy: f64) -> Self {
        let busy = busy.clamp(0.0, 1.0);
        Self {
            user: busy,
            idle: 1.0 - busy,
            ..Self::default()
        }
    }

    pub fn busy(&self) -> f64 {
        (self.user + self.nice + self.system + self.steal).clamp(0.0, 1.0)
    }
}

/// Which parts of the CPU time the system reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Split {
    /// Only how busy the CPU was.
    #[default]
    Busy,
    /// User, nice and system time (macOS).
    Basic,
    /// Also iowait and steal (Linux).
    Full,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cpu {
    /// Every core together, so 8 of 10 cores fully busy is 80%, never 800%.
    pub total: CpuTimes,
    pub cores: Vec<CpuTimes>,
    pub split: Split,
    /// Average current frequency, where the system reports it.
    pub frequency_mhz: Option<u64>,
    /// Degrees Celsius of the CPU die, when a sensor reports it.
    pub temperature: Option<f32>,
    /// Linux pressure stall: percent of the last 10 s some task waited for a CPU.
    pub stall: Option<f64>,
}

/// A slice of the memory bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// Memory of programs (macOS "App Memory").
    App,
    /// Memory the kernel keeps in place.
    Wired,
    /// Memory squeezed by the macOS compressor.
    Compressed,
    /// Memory of programs and the kernel (Linux, as htop counts it).
    Used,
    /// Shared memory and tmpfs (Linux).
    Shared,
    /// Kernel buffers (Linux).
    Buffers,
    /// Files kept in memory; the system gives it back when programs need it.
    Cached,
}

/// How hard macOS works to find free memory, as Activity Monitor shows it.
/// Only macOS reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum PressureLevel {
    Normal,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Memory {
    pub total: u64,
    /// Bytes counted as used: on macOS app, wired and compressed memory, like
    /// Activity Monitor; on Linux what htop counts.
    pub used: u64,
    /// The slices of the bar, in order: used ones first, then buffers and cache.
    pub parts: Vec<(Part, u64)>,
    /// What programs can still get without swapping, when the system says.
    pub available: Option<u64>,
    /// macOS memory pressure.
    pub pressure: Option<PressureLevel>,
    /// Linux pressure stall: percent of the last 10 s some task waited for memory.
    pub stall: Option<f64>,
}

impl Memory {
    pub fn used_ratio(&self) -> f64 {
        ratio(self.used, self.total)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Swap {
    pub total: u64,
    pub used: u64,
    /// Bytes per second read back from swap and written to it, when known.
    pub activity: Option<(f64, f64)>,
}

impl Swap {
    pub fn used_ratio(&self) -> f64 {
        ratio(self.used, self.total)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tasks {
    pub processes: u64,
    pub threads: Option<u64>,
    /// Threads running or ready to run (Linux).
    pub running: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Disk {
    pub mount: String,
    /// Volume name or device: "Macintosh HD", "/dev/nvme0n1p2".
    pub name: String,
    pub file_system: String,
    pub total: u64,
    /// Free space programs can use.
    pub available: u64,
    pub removable: bool,
    /// Bytes per second read and written, when known.
    pub io: Option<(f64, f64)>,
}

impl Disk {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    pub fn used_ratio(&self) -> f64 {
        ratio(self.used(), self.total)
    }
}

/// What stays the same while the program runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Host {
    /// Short host name, without the domain.
    pub name: String,
    /// "macOS 26.5", "Ubuntu 24.04", "Windows 11".
    pub os: String,
    /// "Apple M1 Pro".
    pub cpu: String,
    pub cores: usize,
    /// Core kinds of hybrid chips, fastest first: ("P", 8), ("E", 2).
    pub core_kinds: Vec<(String, usize)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sample {
    pub host: Host,
    pub cpu: Cpu,
    pub memory: Memory,
    pub swap: Swap,
    /// Run queue averaged over 1, 5 and 15 minutes; Windows has none.
    pub load: Option<[f64; 3]>,
    pub tasks: Option<Tasks>,
    /// Seconds since the computer started.
    pub uptime: u64,
    pub disks: Vec<Disk>,
    /// Linux pressure stall: percent of the last 10 s some task waited for the disk.
    pub io_stall: Option<f64>,
}

impl Sample {
    /// The 1-minute load as a share of the cores, so a load of 8 on 10 cores is 80%.
    pub fn load_share(&self) -> Option<f64> {
        let cores = self.host.cores.max(1) as f64;
        self.load.map(|[one, ..]| one / cores)
    }
}

pub fn ratio(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64 / whole as f64).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_leaves_out_idle_and_iowait() {
        let times = CpuTimes {
            user: 0.3,
            nice: 0.1,
            system: 0.2,
            steal: 0.05,
            iowait: 0.15,
            idle: 0.2,
        };
        assert!((times.busy() - 0.65).abs() < 1e-9);
        assert_eq!(CpuTimes::from_busy(1.4).busy(), 1.0);
        assert_eq!(CpuTimes::from_busy(0.25).idle, 0.75);
    }

    #[test]
    fn load_is_shared_among_the_cores() {
        let sample = Sample {
            host: Host {
                cores: 10,
                ..Host::default()
            },
            load: Some([8.0, 5.0, 2.0]),
            ..Sample::default()
        };
        assert_eq!(sample.load_share(), Some(0.8));
    }

    #[test]
    fn ratios_stay_between_zero_and_one() {
        assert_eq!(ratio(5, 0), 0.0);
        assert_eq!(ratio(5, 10), 0.5);
        assert_eq!(ratio(15, 10), 1.0);
        let disk = Disk {
            total: 100,
            available: 25,
            ..Disk::default()
        };
        assert_eq!((disk.used(), disk.used_ratio()), (75, 0.75));
    }
}
