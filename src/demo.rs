//! Made-up readings of a made-up computer, for the tests and the pictures in
//! the README.

use crate::app::App;
use crate::i18n::Lang;
use crate::limits::{Period, Source, Tool, Usage, Window, unix_now};
use crate::model::{
    Cpu, CpuTimes, Disk, Host, Memory, Part, PressureLevel, Sample, Split, Swap, Tasks,
};

const GIB: u64 = 1 << 30;

fn times(busy: f64) -> CpuTimes {
    CpuTimes {
        user: busy * 0.68,
        nice: busy * 0.02,
        system: busy * 0.3,
        idle: 1.0 - busy,
        ..CpuTimes::default()
    }
}

pub fn sample() -> Sample {
    let busy = [
        0.94, 0.88, 0.71, 0.52, 0.37, 0.29, 0.21, 0.14, 0.33, 0.25, 0.12, 0.08,
    ];
    let cores: Vec<CpuTimes> = busy.iter().map(|&busy| times(busy)).collect();
    let average = busy.iter().sum::<f64>() / busy.len() as f64;
    let (app, wired, compressed, cached) = (10 * GIB, 3 * GIB + GIB / 4, 2 * GIB, 5 * GIB);
    Sample {
        host: Host {
            name: "studio".into(),
            os: "macOS 26.0".into(),
            cpu: "Apple M4 Pro".into(),
            cores: busy.len(),
            core_kinds: vec![("P".into(), 8), ("E".into(), 4)],
        },
        cpu: Cpu {
            total: times(average),
            cores,
            split: Split::Basic,
            frequency_mhz: None,
            temperature: Some(52.0),
            stall: None,
        },
        memory: Memory {
            total: 24 * GIB,
            used: app + wired + compressed,
            parts: vec![
                (Part::App, app),
                (Part::Wired, wired),
                (Part::Compressed, compressed),
                (Part::Cached, cached),
            ],
            available: None,
            pressure: Some(PressureLevel::Normal),
            stall: None,
        },
        swap: Swap {
            total: 3 * GIB,
            used: GIB - GIB / 5,
            activity: Some((0.0, 49_152.0)),
        },
        load: Some([4.21, 3.87, 3.02]),
        tasks: Some(Tasks {
            processes: 612,
            threads: Some(3148),
            running: None,
        }),
        uptime: 3 * 86_400 + 5 * 3600 + 17 * 60,
        disks: vec![
            Disk {
                mount: "/".into(),
                name: "Macintosh HD".into(),
                file_system: "apfs".into(),
                total: 994_662_584_320,
                available: 312_475_000_000,
                removable: false,
                io: Some((1_310_720.0, 327_680.0)),
            },
            Disk {
                mount: "/Volumes/Backup".into(),
                name: "Backup".into(),
                file_system: "apfs".into(),
                total: 2_000_189_177_856,
                available: 181_000_000_000,
                removable: true,
                io: Some((0.0, 0.0)),
            },
        ],
        io_stall: None,
    }
}

/// Made-up usage limits: Claude figures asked just now, Codex figures a
/// session saved a while ago.
pub fn limits() -> Vec<Usage> {
    let now = unix_now();
    let window = |period, used, resets_in: i64| Window {
        period,
        used,
        resets_at: Some(now + resets_in),
        model: None,
    };
    vec![
        Usage {
            tool: Tool::Claude,
            plan: Some("Max 5x".into()),
            windows: vec![
                window(Period::Session, 0.57, 92 * 60),
                window(Period::Week, 0.31, 4 * 86_400 + 3 * 3600),
            ],
            source: Source::Live,
            as_of: now,
        },
        Usage {
            tool: Tool::Codex,
            plan: Some("Plus".into()),
            windows: vec![
                window(Period::Session, 0.12, 3 * 3600 + 20 * 60),
                window(Period::Week, 0.86, 2 * 86_400 + 6 * 3600),
            ],
            source: Source::Saved,
            as_of: now - 25 * 60,
        },
    ]
}

/// An app showing the made-up computer, with ten minutes of history.
pub fn app(lang: Lang) -> App {
    let mut app = App::new(lang);
    app.truecolor = true;
    for step in 0..600 {
        let t = f64::from(step);
        let cpu = 0.34 + 0.2 * (t / 23.0).sin() + 0.12 * (t / 6.0).sin() * (t / 41.0).cos();
        app.history.cpu.push(cpu.clamp(0.02, 0.98));
        app.history
            .memory
            .push(0.58 + 0.06 * (t / 90.0).sin() + 0.01 * (t / 5.0).sin());
        app.history.swap.push(0.22 + 0.03 * (t / 200.0).sin());
    }
    app.sample = Some(sample());
    app.limits = limits();
    app
}
