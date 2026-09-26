//! Codex's limits: live from the address its `/status` asks, with its login,
//! or else the last figures a session saved in `~/.codex/sessions`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{Datelike, Local};
use serde_json::Value;

use super::{
    Period, Source, Tool, Usage, Window, files, newest, parse_time, plan_name, retry_after,
};

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
/// Days of session folders searched: a resumed session keeps writing in the
/// folder of the day it started.
const DAYS: i64 = 14;
/// Session files searched, newest first, for one that holds figures.
const LOGS: usize = 5;

type Seen = (PathBuf, SystemTime);

#[derive(Default)]
pub struct Reader {
    live: Option<Usage>,
    /// Unix seconds from which the address may be asked again.
    ask_after: i64,
    /// The newest session file as last read, and the figures found.
    saved: Option<(Seen, Option<Usage>)>,
}

impl Reader {
    pub fn read(&mut self, agent: &ureq::Agent, now: i64) -> Option<Usage> {
        let home = home()?;
        if now >= self.ask_after {
            let (usage, wait) = ask(agent, &home, now);
            if usage.is_some() {
                self.live = usage;
            }
            self.ask_after = now + wait;
        }
        newest(self.live.clone(), self.saved(&home))
    }

    /// The last figures a session saved, read again only when a session file changes.
    fn saved(&mut self, home: &Path) -> Option<Usage> {
        let mut logs = recent_logs(&home.join("sessions"));
        logs.sort_by_key(|log| std::cmp::Reverse(log.1));
        let newest_log = logs.first()?.clone();
        if self
            .saved
            .as_ref()
            .is_none_or(|(seen, _)| *seen != newest_log)
        {
            // A session that just started holds no figures yet: look further back.
            let usage = logs.iter().take(LOGS).find_map(|(path, _)| {
                files::last_line(path, "\"rate_limits\"", |line| {
                    parse_log_line(line).is_some()
                })
                .and_then(|line| parse_log_line(&line))
            });
            self.saved = Some((newest_log, usage));
        }
        self.saved.as_ref().and_then(|(_, usage)| usage.clone())
    }
}

/// Codex's folder: `$CODEX_HOME`, or `~/.codex`, when it exists.
fn home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(files::home()?.join(".codex")))
        .filter(|home| home.is_dir())
}

/// The live figures with Codex's login, and how long to wait before asking again.
fn ask(agent: &ureq::Agent, home: &Path, now: i64) -> (Option<Usage>, i64) {
    let auth = std::fs::read_to_string(home.join("auth.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let Some(auth) = auth else {
        return (None, retry_after(0));
    };
    let tokens = &auth["tokens"];
    let Some(token) = tokens["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
    else {
        return (None, retry_after(0));
    };
    let mut request = agent
        .get(USAGE_URL)
        .header("Authorization", &format!("Bearer {token}"));
    if let Some(account) = tokens["account_id"].as_str() {
        request = request.header("ChatGPT-Account-Id", account);
    }
    let Ok(mut response) = request.call() else {
        return (None, retry_after(0));
    };
    let status = response.status().as_u16();
    let usage = (status == 200)
        .then(|| response.body_mut().read_json::<Value>().ok())
        .flatten()
        .and_then(|body| parse_live(&body, now));
    (usage, retry_after(status))
}

/// The live answer: `rate_limit.primary_window` and `secondary_window`.
fn parse_live(body: &Value, now: i64) -> Option<Usage> {
    let limit = &body["rate_limit"];
    let windows: Vec<Window> = ["primary_window", "secondary_window"]
        .into_iter()
        .filter_map(|key| {
            let window = &limit[key];
            let used = window["used_percent"].as_f64()?;
            let seconds = window["limit_window_seconds"].as_u64()?;
            let resets_at = window["reset_at"].as_i64().or_else(|| {
                window["reset_after_seconds"]
                    .as_i64()
                    .map(|after| now + after)
            });
            Some(Window {
                period: Period::from_minutes(seconds / 60),
                used: (used / 100.0).clamp(0.0, 1.0),
                resets_at,
                model: None,
            })
        })
        .collect();
    (!windows.is_empty()).then(|| Usage {
        tool: Tool::Codex,
        plan: body["plan_type"].as_str().map(plan_name),
        windows,
        source: Source::Live,
        as_of: now,
    })
}

/// A session log line with figures of Codex's own limits (lines about other
/// buckets have none): `payload.rate_limits.primary` and `secondary`.
fn parse_log_line(line: &str) -> Option<Usage> {
    let value: Value = serde_json::from_str(line).ok()?;
    let limits = &value["payload"]["rate_limits"];
    if limits["limit_id"].as_str().is_some_and(|id| id != "codex") {
        return None;
    }
    let as_of = value["timestamp"].as_str().and_then(parse_time)?;
    let windows: Vec<Window> = ["primary", "secondary"]
        .into_iter()
        .filter_map(|key| {
            let window = &limits[key];
            let used = window["used_percent"].as_f64()?;
            let minutes = window["window_minutes"].as_u64()?;
            // Older versions wrote the seconds left instead of the time.
            let resets_at = window["resets_at"].as_i64().or_else(|| {
                window["resets_in_seconds"]
                    .as_i64()
                    .map(|after| as_of + after)
            });
            Some(Window {
                period: Period::from_minutes(minutes),
                used: (used / 100.0).clamp(0.0, 1.0),
                resets_at,
                model: None,
            })
        })
        .collect();
    (!windows.is_empty()).then(|| Usage {
        tool: Tool::Codex,
        plan: limits["plan_type"].as_str().map(plan_name),
        windows,
        source: Source::Saved,
        as_of,
    })
}

/// Session logs of the last two weeks: `sessions/YYYY/MM/DD/rollout-*.jsonl`.
fn recent_logs(sessions: &Path) -> Vec<(PathBuf, SystemTime)> {
    let today = Local::now().date_naive();
    (0..DAYS)
        .filter_map(|days_ago| today.checked_sub_signed(chrono::Duration::days(days_ago)))
        .flat_map(|day| {
            let dir = sessions
                .join(format!("{:04}", day.year()))
                .join(format!("{:02}", day.month()))
                .join(format!("{:02}", day.day()));
            files::listed(&dir, "rollout-", ".jsonl")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_figures_a_session_saved() {
        let line = r#"{"timestamp":"2026-09-24T18:30:23.413Z","type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":40.0,"window_minutes":10080,"resets_at":1790872857},"secondary":null,"plan_type":"self_serve_business_prolite"}}}"#;
        let usage = parse_log_line(line).unwrap();
        assert_eq!(usage.windows.len(), 1, "no session window on this plan");
        let week = &usage.windows[0];
        assert_eq!(
            week.period,
            Period::Week,
            "the first window is the week here"
        );
        assert!((week.used - 0.4).abs() < 1e-9);
        assert_eq!(week.resets_at, Some(1_790_872_857));
        assert_eq!(usage.plan.as_deref(), Some("Business"));
        assert_eq!(usage.as_of, 1_790_274_623);
    }

    #[test]
    fn reads_older_logs_and_skips_other_buckets() {
        let old = r#"{"timestamp":"2026-06-01T10:00:00Z","payload":{"rate_limits":{"primary":{"used_percent":12.5,"window_minutes":300,"resets_in_seconds":600},"secondary":{"used_percent":3,"window_minutes":10080,"resets_in_seconds":86400}}}}"#;
        let usage = parse_log_line(old).unwrap();
        let session = &usage.windows[0];
        assert_eq!(session.period, Period::Session);
        assert_eq!(session.resets_at, Some(usage.as_of + 600));
        assert_eq!(usage.windows[1].period, Period::Week);
        let premium = r#"{"timestamp":"2026-09-24T16:39:48Z","payload":{"rate_limits":{"limit_id":"premium","primary":null,"secondary":null}}}"#;
        assert_eq!(parse_log_line(premium), None);
    }

    #[test]
    fn reads_the_live_answer() {
        let body: Value = serde_json::from_str(
            r#"{"plan_type":"plus","rate_limit":{"allowed":true,
                "primary_window":{"used_percent":23,"limit_window_seconds":18000,"reset_after_seconds":3600,"reset_at":1790370000},
                "secondary_window":{"used_percent":61,"limit_window_seconds":604800,"reset_after_seconds":400000}}}"#,
        )
        .unwrap();
        let usage = parse_live(&body, 1_790_366_400).unwrap();
        assert_eq!(usage.plan.as_deref(), Some("Plus"));
        assert_eq!(usage.windows[0].period, Period::Session);
        assert_eq!(usage.windows[0].resets_at, Some(1_790_370_000));
        assert_eq!(usage.windows[1].period, Period::Week);
        assert_eq!(usage.windows[1].resets_at, Some(1_790_366_400 + 400_000));
    }
}
