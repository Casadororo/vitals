//! How much of their usage limits the AI coding tools on this computer have
//! used: the 5-hour session and the week, and when each starts over.
//!
//! The live figures come from the addresses the tools themselves ask, with
//! the login they keep on this computer; vitals only reads that login and
//! never renews it, so it cannot log the tool out. When a login is missing or
//! expired, the last figures the tool saved on disk stand in.

mod claude;
mod codex;
mod files;

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How often the figures are read again. The addresses are asked less often.
const EVERY: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Claude,
    Codex,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
        }
    }
}

/// How long a limit counts usage before it starts over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    /// The 5-hour window a session opens.
    Session,
    Week,
    Month,
    Minutes(u64),
}

impl Period {
    /// Windows are told apart by their length, never by their order: Codex
    /// lists the week first on plans without a session window.
    pub fn from_minutes(minutes: u64) -> Self {
        match minutes {
            300 => Self::Session,
            10_080 => Self::Week,
            40_000..=44_700 => Self::Month,
            other => Self::Minutes(other),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub period: Period,
    /// Share used, from 0 to 1.
    pub used: f64,
    /// Unix seconds when it starts over.
    pub resets_at: Option<i64>,
    /// The model a separate weekly limit counts: "Opus".
    pub model: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Asked just now at the tool's own address.
    Live,
    /// Saved by the tool on disk.
    Saved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Usage {
    pub tool: Tool,
    /// "Max 20x", "Plus".
    pub plan: Option<String>,
    pub windows: Vec<Window>,
    pub source: Source,
    /// Unix seconds of the figures.
    pub as_of: i64,
}

/// Starts reading the limits on a thread of its own, sending them out every
/// 30 seconds while they are `wanted`; `changes` switches that, and switching
/// it on reads at once. The thread ends with the channel.
pub fn spawn<T: Send + 'static>(
    sender: Sender<T>,
    wrap: fn(Vec<Usage>) -> T,
    changes: Receiver<bool>,
    mut wanted: bool,
) {
    thread::spawn(move || {
        let agent = agent();
        let mut readers = Readers::default();
        loop {
            if wanted && sender.send(wrap(readers.read(&agent))).is_err() {
                return;
            }
            match changes.recv_timeout(EVERY) {
                Ok(now_wanted) => wanted = now_wanted,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    });
}

/// Reads the limits once, for a snapshot.
pub fn read_once() -> Vec<Usage> {
    Readers::default().read(&agent())
}

#[derive(Default)]
struct Readers {
    claude: claude::Reader,
    codex: codex::Reader,
}

impl Readers {
    fn read(&mut self, agent: &ureq::Agent) -> Vec<Usage> {
        let now = unix_now();
        [self.claude.read(agent, now), self.codex.read(agent, now)]
            .into_iter()
            .flatten()
            .collect()
    }
}

/// The HTTP client: the system's trust store, like browsers and curl, and
/// every answer returned, errors included, to decide when to ask again.
fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .http_status_as_error(false)
        .user_agent(concat!("vitals/", env!("CARGO_PKG_VERSION")))
        .tls_config(tls)
        .build()
        .into()
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}

/// The newer of two readings.
fn newest(a: Option<Usage>, b: Option<Usage>) -> Option<Usage> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if b.as_of > a.as_of { b } else { a }),
        (a, b) => a.or(b),
    }
}

/// Seconds to wait before asking a tool's address again, after an answer
/// with this status.
fn retry_after(status: u16) -> i64 {
    match status {
        200..=299 => 180,
        // The login expired: the tool renews it the next time it runs.
        401 | 403 => 300,
        429 => 900,
        _ => 300,
    }
}

/// An RFC 3339 time as Unix seconds: "2026-09-25T20:09:59.699485+00:00".
fn parse_time(text: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|time| time.timestamp())
}

/// A plan's name as people call it: "self_serve_business_prolite" is "Business".
fn plan_name(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    for (key, name) in [
        ("enterprise", "Enterprise"),
        ("business", "Business"),
        ("team", "Team"),
        ("edu", "Edu"),
        ("plus", "Plus"),
        ("pro", "Pro"),
        ("max", "Max"),
        ("free", "Free"),
    ] {
        if lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == key)
        {
            return name.to_owned();
        }
    }
    let mut chars = raw.chars();
    chars
        .next()
        .map(|first| {
            first
                .to_uppercase()
                .chain(chars)
                .collect::<String>()
                .replace('_', " ")
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_windows_apart_by_their_length() {
        assert_eq!(Period::from_minutes(300), Period::Session);
        assert_eq!(Period::from_minutes(10_080), Period::Week);
        assert_eq!(Period::from_minutes(43_200), Period::Month);
        assert_eq!(Period::from_minutes(60), Period::Minutes(60));
    }

    #[test]
    fn keeps_the_newer_reading() {
        let usage = |as_of| Usage {
            tool: Tool::Claude,
            plan: None,
            windows: Vec::new(),
            source: Source::Saved,
            as_of,
        };
        assert_eq!(newest(Some(usage(1)), Some(usage(2))), Some(usage(2)));
        assert_eq!(newest(Some(usage(3)), Some(usage(2))), Some(usage(3)));
        assert_eq!(newest(None, Some(usage(2))), Some(usage(2)));
        assert_eq!(newest(None, None), None);
    }

    #[test]
    fn names_plans_as_people_call_them() {
        assert_eq!(plan_name("self_serve_business_prolite"), "Business");
        assert_eq!(plan_name("plus"), "Plus");
        assert_eq!(plan_name("pro"), "Pro");
        assert_eq!(plan_name("prolite"), "Prolite");
        assert_eq!(plan_name("max"), "Max");
    }

    #[test]
    fn reads_rfc3339_times() {
        assert_eq!(
            parse_time("2026-09-25T20:09:59.699485+00:00"),
            Some(1_790_366_999)
        );
        assert_eq!(parse_time("yesterday"), None);
    }
}
