//! Parsers for the Linux files under /proc. They take the text, so every
//! system can test them; only Linux reads the files.

use std::collections::HashMap;

use super::Ticks;
use crate::model::{Memory, Part, Swap, Tasks};

/// Per-core time counters from /proc/stat, in the order of the cores.
pub fn stat_cores(text: &str) -> Vec<Ticks> {
    text.lines()
        .filter(|line| {
            line.strip_prefix("cpu")
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        })
        .filter_map(|line| {
            let values: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .map(|value| value.parse().ok())
                .collect::<Option<_>>()?;
            // user nice system idle iowait irq softirq steal guest guest_nice;
            // guest time is already inside user and nice.
            let get = |index: usize| values.get(index).copied().unwrap_or(0);
            Some(Ticks {
                user: get(0),
                nice: get(1),
                system: get(2) + get(5) + get(6),
                idle: get(3),
                iowait: get(4),
                steal: get(7),
            })
        })
        .collect()
}

/// The fields of /proc/meminfo, in bytes.
pub fn meminfo(text: &str) -> HashMap<&str, u64> {
    text.lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(':')?;
            let mut words = rest.split_whitespace();
            let value: u64 = words.next()?.parse().ok()?;
            let bytes = match words.next() {
                Some("kB") => value * 1024,
                _ => value,
            };
            Some((name.trim(), bytes))
        })
        .collect()
}

/// Memory as htop counts it: used is what is neither free, nor buffers, nor
/// cache; shared memory (tmpfs) is used too. Also the swap totals.
pub fn memory(info: &HashMap<&str, u64>) -> Option<(Memory, Swap)> {
    let total = *info.get("MemTotal")?;
    let get = |name: &str| info.get(name).copied().unwrap_or(0);
    let free = get("MemFree");
    let buffers = get("Buffers");
    let shared = get("Shmem");
    let cached = get("Cached") + get("SReclaimable");
    let used = total
        .checked_sub(free + buffers + cached)
        .unwrap_or_else(|| total.saturating_sub(free));
    let cache = cached.saturating_sub(shared);
    let available = info
        .get("MemAvailable")
        .map_or(free, |&available| available.min(total));
    let memory = Memory {
        total,
        used: (used + shared).min(total),
        parts: vec![
            (Part::Used, used),
            (Part::Shared, shared),
            (Part::Buffers, buffers),
            (Part::Cached, cache),
        ],
        available: Some(available),
        pressure: None,
        stall: None,
    };
    let swap_total = get("SwapTotal");
    let swap = Swap {
        total: swap_total,
        used: swap_total.saturating_sub(get("SwapFree")),
        activity: None,
    };
    Some((memory, swap))
}

/// /proc/loadavg: the three load averages and the running and total threads.
pub fn loadavg(text: &str) -> Option<([f64; 3], Tasks)> {
    let mut fields = text.split_whitespace();
    let mut load = [0.0; 3];
    for value in &mut load {
        *value = fields.next()?.parse().ok()?;
    }
    let (running, threads) = fields.next()?.split_once('/')?;
    let tasks = Tasks {
        processes: 0,
        threads: threads.parse().ok(),
        running: running.parse().ok(),
    };
    Some((load, tasks))
}

/// The "some avg10" figure of a /proc/pressure file: percent of the last
/// 10 seconds in which some task waited.
pub fn pressure(text: &str) -> Option<f64> {
    let line = text.lines().find(|line| line.starts_with("some "))?;
    line.split_whitespace()
        .find_map(|field| field.strip_prefix("avg10="))?
        .parse()
        .ok()
}

/// Pages read back from swap and written to it since boot, from /proc/vmstat.
pub fn vmstat_swap(text: &str) -> Option<(u64, u64)> {
    let mut swapped_in = None;
    let mut swapped_out = None;
    for line in text.lines() {
        match line.split_once(' ') {
            Some(("pswpin", value)) => swapped_in = value.trim().parse().ok(),
            Some(("pswpout", value)) => swapped_out = value.trim().parse().ok(),
            _ => {}
        }
    }
    Some((swapped_in?, swapped_out?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &str = "\
cpu  4705 356 584 3699 23 23 0 0 0 0
cpu0 1393280 32966 572056 13343292 6130 0 17875 0 23933 0
cpu1 1335 20 101 900 3 7 8 11 0 0
intr 114930548 113199788 3 0 5 263 0 4 [...]
ctxt 1990473
procs_running 2
";

    #[test]
    fn reads_the_ticks_of_each_core() {
        let cores = stat_cores(STAT);
        assert_eq!(cores.len(), 2, "the all-cores line is left out");
        assert_eq!(
            cores[1],
            Ticks {
                user: 1335,
                nice: 20,
                system: 101 + 7 + 8,
                idle: 900,
                iowait: 3,
                steal: 11,
            }
        );
    }

    #[test]
    fn reads_old_kernels_with_fewer_columns() {
        let cores = stat_cores("cpu0 10 20 30 40\n");
        assert_eq!(
            cores,
            [Ticks {
                user: 10,
                nice: 20,
                system: 30,
                idle: 40,
                ..Ticks::default()
            }]
        );
    }

    const MEMINFO: &str = "\
MemTotal:       16000000 kB
MemFree:         2000000 kB
MemAvailable:    9000000 kB
Buffers:          500000 kB
Cached:          6000000 kB
SwapCached:        10000 kB
Shmem:            400000 kB
SReclaimable:     300000 kB
SwapTotal:       4000000 kB
SwapFree:        3000000 kB
HugePages_Total:       0
";

    #[test]
    fn counts_memory_like_htop() {
        let info = meminfo(MEMINFO);
        assert_eq!(info["HugePages_Total"], 0);
        let (memory, swap) = memory(&info).unwrap();
        let kb = |value: u64| value * 1024;
        assert_eq!(memory.total, kb(16_000_000));
        // Used 16 - 2 free - 0.5 buffers - 6.3 cache = 7.2 GB, plus 0.4 shared.
        assert_eq!(memory.used, kb(7_200_000 + 400_000));
        assert_eq!(
            memory.parts,
            vec![
                (Part::Used, kb(7_200_000)),
                (Part::Shared, kb(400_000)),
                (Part::Buffers, kb(500_000)),
                (Part::Cached, kb(5_900_000)),
            ]
        );
        assert_eq!(memory.available, Some(kb(9_000_000)));
        assert_eq!((swap.total, swap.used), (kb(4_000_000), kb(1_000_000)));
    }

    #[test]
    fn memory_needs_a_total() {
        assert!(memory(&meminfo("MemFree: 10 kB\n")).is_none());
    }

    #[test]
    fn reads_the_load_and_the_threads() {
        let (load, tasks) = loadavg("0.52 1.58 12.00 3/1234 56789\n").unwrap();
        assert_eq!(load, [0.52, 1.58, 12.0]);
        assert_eq!((tasks.running, tasks.threads), (Some(3), Some(1234)));
        assert!(loadavg("garbage").is_none());
    }

    #[test]
    fn reads_the_share_of_time_spent_waiting() {
        let text = "some avg10=1.25 avg60=0.50 avg300=0.10 total=12345\n\
                    full avg10=0.30 avg60=0.10 avg300=0.00 total=2345\n";
        assert_eq!(pressure(text), Some(1.25));
        assert_eq!(pressure(""), None);
    }

    #[test]
    fn reads_the_swap_page_counters() {
        let text = "nr_free_pages 1234\npswpin 42\npswpout 1337\n";
        assert_eq!(vmstat_swap(text), Some((42, 1337)));
        assert_eq!(vmstat_swap("pswpin 1\n"), None);
    }
}
