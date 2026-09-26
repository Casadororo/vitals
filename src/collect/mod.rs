//! Reads the computer. On macOS and Linux the CPU time and the memory come
//! from the system itself, split the way Activity Monitor and htop split
//! them; elsewhere sysinfo gives the totals. Readings happen on a thread of
//! their own, so a slow disk never freezes the screen.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod procfs;

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::{
    Components, CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System,
};

use crate::model::{Cpu, CpuTimes, Disk, Host, Memory, Part, Sample, Split, Swap, Tasks};

/// Parts of the CPU time the native readers report.
const NATIVE_SPLIT: Split = if cfg!(target_os = "linux") {
    Split::Full
} else {
    Split::Basic
};

/// Disk space, sensors and the process list change slowly and cost more to read.
const SLOW_EVERY: Duration = Duration::from_secs(10);
/// The first reading comes quickly, so the screen fills at once.
const FIRST_READING: Duration = Duration::from_millis(250);

/// Cumulative CPU time counters of one core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ticks {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub steal: u64,
    pub iowait: u64,
    pub idle: u64,
}

impl Ticks {
    fn plus(self, other: Self) -> Self {
        Self {
            user: self.user + other.user,
            nice: self.nice + other.nice,
            system: self.system + other.system,
            steal: self.steal + other.steal,
            iowait: self.iowait + other.iowait,
            idle: self.idle + other.idle,
        }
    }
}

/// How each core, and all of them together, spent the time between two
/// readings. All cores together is the sum of their time, so 8 of 10 cores
/// fully busy is 80%.
pub fn cpu_times(before: &[Ticks], after: &[Ticks]) -> (CpuTimes, Vec<CpuTimes>) {
    let shares = |before: Ticks, after: Ticks| {
        let spent = |from: u64, to: u64| to.saturating_sub(from) as f64;
        let parts = [
            spent(before.user, after.user),
            spent(before.nice, after.nice),
            spent(before.system, after.system),
            spent(before.steal, after.steal),
            spent(before.iowait, after.iowait),
            spent(before.idle, after.idle),
        ];
        let total: f64 = parts.iter().sum();
        if total <= 0.0 {
            return CpuTimes {
                idle: 1.0,
                ..CpuTimes::default()
            };
        }
        let [user, nice, system, steal, iowait, idle] = parts.map(|part| part / total);
        CpuTimes {
            user,
            nice,
            system,
            steal,
            iowait,
            idle,
        }
    };
    let cores = before
        .iter()
        .zip(after)
        .map(|(&before, &after)| shares(before, after))
        .collect();
    let sum = |ticks: &[Ticks]| ticks.iter().fold(Ticks::default(), |sum, &t| sum.plus(t));
    (shares(sum(before), sum(after)), cores)
}

/// Bytes per second.
pub fn rate(bytes: u64, seconds: f64) -> f64 {
    if seconds > 0.0 {
        bytes as f64 / seconds
    } else {
        0.0
    }
}

pub struct Collector {
    host: Host,
    system: System,
    disks: Disks,
    components: Components,
    ticks: Option<Vec<Ticks>>,
    /// Disks whose I/O counters were read before, and when.
    io_seen: HashSet<String>,
    io_read_at: Option<Instant>,
    slow_read_at: Option<Instant>,
    temperature: Option<f32>,
    /// Process count on systems without a cheaper way to get it.
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    processes: Option<u64>,
    #[cfg(target_os = "macos")]
    mac: macos::Mac,
    #[cfg(target_os = "linux")]
    linux: linux::Linux,
}

impl Collector {
    pub fn new() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::everything()),
        );
        let mut collector = Self {
            host: host(&system),
            system,
            disks: Disks::new(),
            components: Components::new(),
            ticks: None,
            io_seen: HashSet::new(),
            io_read_at: None,
            slow_read_at: None,
            temperature: None,
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            processes: None,
            #[cfg(target_os = "macos")]
            mac: macos::Mac::new(),
            #[cfg(target_os = "linux")]
            linux: linux::Linux::new(),
        };
        // The CPU share is measured between two readings: this is the first.
        collector.ticks = collector.read_ticks();
        collector
    }

    /// Reads everything once. Disk space, sensors and the process list are
    /// read every 10 seconds; the rest every time.
    pub fn sample(&mut self) -> Sample {
        let now = Instant::now();
        let slow = self
            .slow_read_at
            .is_none_or(|then| now.duration_since(then) >= SLOW_EVERY);
        if slow {
            self.slow_read_at = Some(now);
            self.components.refresh(true);
            self.temperature = cpu_temperature(
                self.components
                    .iter()
                    .filter_map(|sensor| Some((sensor.label(), sensor.temperature()?))),
            );
        }
        let (memory, swap) = self.memory(now);
        Sample {
            host: self.host.clone(),
            cpu: self.cpu(),
            memory,
            swap,
            load: load(),
            tasks: self.tasks(slow),
            uptime: System::uptime(),
            disks: self.disks(now, slow),
            io_stall: self.stall("io"),
        }
    }

    #[cfg(target_os = "macos")]
    fn read_ticks(&mut self) -> Option<Vec<Ticks>> {
        self.mac.ticks()
    }

    #[cfg(target_os = "linux")]
    fn read_ticks(&mut self) -> Option<Vec<Ticks>> {
        self.linux.ticks()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn read_ticks(&mut self) -> Option<Vec<Ticks>> {
        None
    }

    #[cfg(target_os = "linux")]
    fn stall(&self, resource: &str) -> Option<f64> {
        linux::stall(resource)
    }

    #[cfg(not(target_os = "linux"))]
    fn stall(&self, _resource: &str) -> Option<f64> {
        None
    }

    fn cpu(&mut self) -> Cpu {
        let (total, cores, split) = match (self.ticks.take(), self.read_ticks()) {
            (Some(before), Some(after)) if before.len() == after.len() => {
                let (total, cores) = cpu_times(&before, &after);
                self.ticks = Some(after);
                (total, cores, NATIVE_SPLIT)
            }
            (_, Some(after)) => {
                let idle = vec![CpuTimes::from_busy(0.0); after.len()];
                self.ticks = Some(after);
                (CpuTimes::from_busy(0.0), idle, NATIVE_SPLIT)
            }
            (_, None) => {
                self.system.refresh_cpu_usage();
                let cores = self
                    .system
                    .cpus()
                    .iter()
                    .map(|cpu| CpuTimes::from_busy(f64::from(cpu.cpu_usage()) / 100.0))
                    .collect();
                let total = f64::from(self.system.global_cpu_usage()) / 100.0;
                (CpuTimes::from_busy(total), cores, Split::Busy)
            }
        };
        Cpu {
            total,
            cores,
            split,
            frequency_mhz: self.frequency(),
            temperature: self.temperature,
            stall: self.stall("cpu"),
        }
    }

    /// Average current frequency. Apple silicon reports only the top one, which says nothing.
    fn frequency(&mut self) -> Option<u64> {
        if cfg!(target_os = "macos") {
            return None;
        }
        self.system.refresh_cpu_frequency();
        let speeds: Vec<u64> = self
            .system
            .cpus()
            .iter()
            .map(sysinfo::Cpu::frequency)
            // Virtual machines report nonsense; no real CPU runs this slow.
            .filter(|&mhz| mhz >= 100)
            .collect();
        (!speeds.is_empty()).then(|| speeds.iter().sum::<u64>() / speeds.len() as u64)
    }

    fn memory(&mut self, _now: Instant) -> (Memory, Swap) {
        #[cfg(target_os = "macos")]
        if let Some(reading) = self.mac.memory(_now) {
            return reading;
        }
        #[cfg(target_os = "linux")]
        if let Some(reading) = self.linux.memory(_now) {
            return reading;
        }
        self.system.refresh_memory();
        let total = self.system.total_memory();
        let available = self.system.available_memory().min(total);
        let used = total - available;
        let memory = Memory {
            total,
            used,
            parts: vec![(Part::Used, used)],
            available: Some(available),
            ..Memory::default()
        };
        let swap = Swap {
            total: self.system.total_swap(),
            used: self.system.used_swap().min(self.system.total_swap()),
            activity: None,
        };
        (memory, swap)
    }

    #[cfg(target_os = "macos")]
    fn tasks(&mut self, _slow: bool) -> Option<Tasks> {
        self.mac.tasks()
    }

    #[cfg(target_os = "linux")]
    fn tasks(&mut self, _slow: bool) -> Option<Tasks> {
        self.linux.tasks()
    }

    /// Elsewhere the process list is the only count, read every 10 seconds.
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn tasks(&mut self, slow: bool) -> Option<Tasks> {
        if slow {
            self.system.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::All,
                true,
                sysinfo::ProcessRefreshKind::nothing(),
            );
            self.processes = Some(self.system.processes().len() as u64);
        }
        self.processes.map(|processes| Tasks {
            processes,
            ..Tasks::default()
        })
    }

    fn disks(&mut self, now: Instant, slow: bool) -> Vec<Disk> {
        let kind = if slow {
            DiskRefreshKind::everything()
        } else {
            DiskRefreshKind::nothing().with_io_usage()
        };
        self.disks.refresh_specifics(true, kind);
        let seconds = self
            .io_read_at
            .map(|then| now.duration_since(then).as_secs_f64());
        self.io_read_at = Some(now);
        let mut seen = HashSet::new();
        let disks = self
            .disks
            .iter()
            .map(|disk| {
                let mount = disk.mount_point().to_string_lossy().into_owned();
                let usage = disk.usage();
                // A disk read for the first time reports its counters since boot.
                let io = seconds
                    .filter(|_| self.io_seen.contains(&mount))
                    .map(|seconds| {
                        (
                            rate(usage.read_bytes, seconds),
                            rate(usage.written_bytes, seconds),
                        )
                    });
                seen.insert(mount.clone());
                Disk {
                    mount,
                    name: disk.name().to_string_lossy().into_owned(),
                    file_system: disk.file_system().to_string_lossy().into_owned(),
                    total: disk.total_space(),
                    available: disk.available_space().min(disk.total_space()),
                    removable: disk.is_removable(),
                    io,
                }
            })
            .collect();
        self.io_seen = seen;
        shown_disks(disks)
    }
}

/// Starts reading on a thread of its own: a reading soon, then one every
/// interval. Each goes out through `send`; a new interval arriving on
/// `intervals` takes a reading at once. The thread ends with the channel.
pub fn spawn<T: Send + 'static>(
    sender: Sender<T>,
    wrap: fn(Sample) -> T,
    intervals: Receiver<Duration>,
    mut interval: Duration,
) {
    thread::spawn(move || {
        let mut collector = Collector::new();
        let mut wait = FIRST_READING;
        loop {
            match intervals.recv_timeout(wait) {
                Ok(new) => interval = new,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            if sender.send(wrap(collector.sample())).is_err() {
                return;
            }
            wait = interval;
        }
    });
}

/// The 1, 5 and 15 minute load averages; Windows has none.
fn load() -> Option<[f64; 3]> {
    if cfg!(windows) {
        return None;
    }
    let load = System::load_average();
    Some([load.one, load.five, load.fifteen])
}

fn host(system: &System) -> Host {
    let name = System::host_name().unwrap_or_default();
    Host {
        name: name.split('.').next().unwrap_or_default().to_owned(),
        os: os_name(System::long_os_version().unwrap_or_default()),
        cpu: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_owned())
            .unwrap_or_default(),
        cores: system.cpus().len().max(1),
        #[cfg(target_os = "macos")]
        core_kinds: macos::core_kinds(),
        #[cfg(not(target_os = "macos"))]
        core_kinds: Vec::new(),
    }
}

/// "Linux (Ubuntu 24.04)" reads better as "Ubuntu 24.04".
fn os_name(long: String) -> String {
    match long
        .strip_prefix("Linux (")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        Some(distribution) => distribution.to_owned(),
        None => long,
    }
}

/// The disks worth showing, the root first: no empty volumes and no disk
/// images of the iOS simulators, and one entry per storage pool, since the
/// volumes of an APFS container (or btrfs subvolumes) share their space.
pub fn shown_disks(mut disks: Vec<Disk>) -> Vec<Disk> {
    disks.retain(|disk| {
        disk.total > 0 && !disk.mount.starts_with("/Library/Developer/CoreSimulator/")
    });
    disks.sort_by(|a, b| {
        (a.mount != "/", a.mount.len(), &a.mount).cmp(&(b.mount != "/", b.mount.len(), &b.mount))
    });
    let mut pools = HashSet::new();
    disks.retain(|disk| pools.insert((disk.name.clone(), disk.file_system.clone(), disk.total)));
    disks.sort_by(|a, b| (a.mount != "/", &a.mount).cmp(&(b.mount != "/", &b.mount)));
    disks
}

/// The CPU temperature among the sensors: the package or die sensor, the
/// average of Apple silicon's die sensors, or the average of the cores.
pub fn cpu_temperature<'a>(sensors: impl IntoIterator<Item = (&'a str, f32)>) -> Option<f32> {
    let sensors: Vec<(String, f32)> = sensors
        .into_iter()
        .filter(|(_, celsius)| celsius.is_finite() && (1.0..150.0).contains(celsius))
        .map(|(label, celsius)| (label.to_ascii_lowercase(), celsius))
        .collect();
    const PREFERENCE: [&[&str]; 4] = [
        &[
            "package id",
            "tctl",
            "tdie",
            "x86_pkg_temp",
            "cpu_thermal",
            "cpu package",
        ],
        &["pmu tdie"],
        &["coretemp core", "core "],
        &["cpu"],
    ];
    PREFERENCE.iter().find_map(|names| {
        let found: Vec<f32> = sensors
            .iter()
            .filter(|(label, _)| names.iter().any(|name| label.contains(name)))
            .map(|&(_, celsius)| celsius)
            .collect();
        (!found.is_empty()).then(|| found.iter().sum::<f32>() / found.len() as f32)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(user: u64, system: u64, idle: u64) -> Ticks {
        Ticks {
            user,
            system,
            idle,
            ..Ticks::default()
        }
    }

    #[test]
    fn eight_of_ten_busy_cores_are_eighty_percent() {
        let before = vec![ticks(0, 0, 0); 10];
        let mut after = vec![ticks(100, 0, 0); 8];
        after.extend([ticks(0, 0, 100); 2]);
        let (total, cores) = cpu_times(&before, &after);
        assert!((total.busy() - 0.8).abs() < 1e-9, "{}", total.busy());
        assert_eq!(cores.len(), 10);
        assert_eq!(cores[0].busy(), 1.0);
        assert_eq!(cores[9].busy(), 0.0);
    }

    #[test]
    fn splits_the_time_like_htop() {
        let before = [Ticks::default()];
        let after = [Ticks {
            user: 50,
            nice: 10,
            system: 20,
            steal: 5,
            iowait: 5,
            idle: 10,
        }];
        let (total, _) = cpu_times(&before, &after);
        assert_eq!(
            (total.user, total.nice, total.system, total.iowait),
            (0.5, 0.1, 0.2, 0.05)
        );
        assert!((total.busy() - 0.85).abs() < 1e-9);
    }

    #[test]
    fn a_core_without_ticks_counts_as_idle() {
        let (total, cores) = cpu_times(&[ticks(5, 5, 5)], &[ticks(5, 5, 5)]);
        assert_eq!((total.busy(), cores[0].idle), (0.0, 1.0));
        // Counters that went back (a core restarted) are not negative time.
        let (total, _) = cpu_times(&[ticks(50, 0, 0)], &[ticks(10, 0, 30)]);
        assert_eq!(total.busy(), 0.0);
    }

    fn disk(mount: &str, name: &str, total: u64, available: u64) -> Disk {
        Disk {
            mount: mount.into(),
            name: name.into(),
            file_system: "apfs".into(),
            total,
            available,
            ..Disk::default()
        }
    }

    #[test]
    fn shows_each_apfs_container_once_with_the_root_first() {
        let shown = shown_disks(vec![
            disk("/Volumes/Backup", "Backup", 2_000, 500),
            disk("/System/Volumes/Data", "Macintosh HD", 1_000, 300),
            disk("/", "Macintosh HD", 1_000, 301),
            disk("/Volumes/Empty", "Empty", 0, 0),
            disk(
                "/Library/Developer/CoreSimulator/Volumes/iOS_22A",
                "iOS",
                9_000,
                0,
            ),
            disk("/Volumes/Archive", "Archive", 3_000, 100),
        ]);
        let mounts: Vec<&str> = shown.iter().map(|disk| disk.mount.as_str()).collect();
        assert_eq!(mounts, ["/", "/Volumes/Archive", "/Volumes/Backup"]);
    }

    #[test]
    fn keeps_different_disks_of_the_same_size() {
        let shown = shown_disks(vec![
            disk("/mnt/a", "/dev/sda1", 1_000, 10),
            disk("/mnt/b", "/dev/sdb1", 1_000, 10),
        ]);
        assert_eq!(shown.len(), 2);
    }

    #[test]
    fn finds_the_cpu_among_the_sensors() {
        let apple = [
            ("PMU tdie1", 40.0),
            ("PMU tdie2", 44.0),
            ("gas gauge battery", 30.0),
            ("NAND CH0 temp", 35.0),
        ];
        assert_eq!(cpu_temperature(apple), Some(42.0));
        let intel = [
            ("coretemp Core 0", 50.0),
            ("coretemp Package id 0", 55.0),
            ("acpitz temp1", 27.8),
        ];
        assert_eq!(cpu_temperature(intel), Some(55.0));
        let amd = [("k10temp Tctl", 61.5), ("nvme Composite", 40.0)];
        assert_eq!(cpu_temperature(amd), Some(61.5));
        assert_eq!(cpu_temperature([("nvme Composite", 40.0)]), None);
        assert_eq!(cpu_temperature([("cpu", f32::NAN)]), None);
    }

    #[test]
    fn names_the_linux_distribution() {
        assert_eq!(os_name("Linux (Ubuntu 24.04)".into()), "Ubuntu 24.04");
        assert_eq!(os_name("macOS 26.5".into()), "macOS 26.5");
    }

    #[test]
    fn rates_need_time_to_pass() {
        assert_eq!(rate(1000, 2.0), 500.0);
        assert_eq!(rate(1000, 0.0), 0.0);
    }

    /// Reads this computer, whatever it is.
    #[test]
    fn reads_this_computer() {
        let mut collector = Collector::new();
        thread::sleep(Duration::from_millis(100));
        let sample = collector.sample();
        assert!(sample.host.cores >= 1);
        assert_eq!(sample.cpu.cores.len(), sample.host.cores);
        assert!((0.0..=1.0).contains(&sample.cpu.total.busy()));
        assert!(sample.memory.total > 0);
        assert!(sample.memory.used <= sample.memory.total);
        assert!(sample.swap.used <= sample.swap.total);
        assert!(!sample.disks.is_empty(), "no disk at all");
        for disk in &sample.disks {
            assert!(disk.total > 0 && disk.available <= disk.total, "{disk:?}");
        }
        if cfg!(unix) {
            assert!(sample.load.is_some());
            // In a container this test can be the only process there is.
            assert!(sample.tasks.is_some_and(|tasks| tasks.processes >= 1));
        }
    }
}
