//! Claude Code's limits: live from the address its `/usage` asks, with its
//! login, or else the last figures it saved in `~/.claude.json`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::Value;

use super::{Period, Source, Tool, Usage, Window, files, newest, parse_time, retry_after};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The beta header Claude Code sends with its login.
const OAUTH_BETA: &str = "oauth-2025-04-20";

#[derive(Default)]
pub struct Reader {
    /// The folder given with `--claude-dir` or found beside `~/.claude`; none
    /// is Claude Code's own.
    dir: Option<PathBuf>,
    /// The account's name, from its folder: "2" for `~/.claude-2`.
    account: Option<String>,
    live: Option<Usage>,
    /// Unix seconds from which the address may be asked again.
    ask_after: i64,
    /// The plan, known from the login.
    plan: Option<String>,
    /// `~/.claude.json` as last read: when it changed, and its figures.
    saved: Option<(SystemTime, Option<Usage>)>,
}

impl Reader {
    /// A reader of the Claude Code folder given, such as `~/.claude-2`.
    pub fn new(dir: PathBuf) -> Self {
        Self {
            account: account(&dir),
            dir: Some(dir),
            ..Self::default()
        }
    }

    pub fn read(&mut self, agent: &ureq::Agent, now: i64) -> Option<Usage> {
        if now >= self.ask_after {
            let (usage, wait) = self.ask(agent, now);
            if usage.is_some() {
                self.live = usage;
            }
            self.ask_after = now + wait;
        }
        let mut saved = self.saved();
        if let Some(saved) = &mut saved {
            saved.plan = self.plan.clone();
        }
        let account = self.account.clone();
        newest(self.live.clone(), saved).map(|usage| Usage { account, ..usage })
    }

    /// The live figures, and how long to wait before asking again.
    fn ask(&mut self, agent: &ureq::Agent, now: i64) -> (Option<Usage>, i64) {
        let Some(login) = login(self.dir.as_deref()) else {
            return (None, retry_after(0));
        };
        self.plan = login.plan.clone();
        // Claude Code renews its login while it runs; vitals never does.
        if login.expires_at <= (now + 60) * 1000 {
            return (None, 60);
        }
        let answer = agent
            .get(USAGE_URL)
            .header("Authorization", &format!("Bearer {}", login.token))
            .header("anthropic-beta", OAUTH_BETA)
            .header("Content-Type", "application/json")
            .call();
        let Ok(mut response) = answer else {
            return (None, retry_after(0));
        };
        let status = response.status().as_u16();
        let usage = (status == 200)
            .then(|| response.body_mut().read_json::<Value>().ok())
            .flatten()
            .and_then(|body| parse(&body, Source::Live, now, login.plan));
        (usage, retry_after(status))
    }

    /// The figures Claude Code saved the last time it showed them, read again
    /// only when the file changes.
    fn saved(&mut self) -> Option<Usage> {
        let path = global_config(self.dir.as_deref())?;
        let changed = files::modified(&path)?;
        if self.saved.as_ref().is_none_or(|(seen, _)| *seen != changed) {
            let usage = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .and_then(|config| saved_usage(&config));
            self.saved = Some((changed, usage));
        }
        self.saved.as_ref().and_then(|(_, usage)| usage.clone())
    }
}

/// Claude Code's folder: the one given, `$CLAUDE_CONFIG_DIR`, or `~/.claude`.
fn config_dir(given: Option<&Path>) -> Option<PathBuf> {
    match given {
        Some(dir) => Some(dir.to_path_buf()),
        None => std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| Some(files::home()?.join(".claude"))),
    }
}

/// Claude Code's global settings: `.claude.json` in the folder given or in
/// `$CLAUDE_CONFIG_DIR`, but `~/.claude.json` for the default `~/.claude`.
fn global_config(given: Option<&Path>) -> Option<PathBuf> {
    match given {
        Some(dir) if !is_default(dir) => Some(dir.join(".claude.json")),
        Some(_) => Some(files::home()?.join(".claude.json")),
        None => match std::env::var_os("CLAUDE_CONFIG_DIR") {
            Some(dir) => Some(PathBuf::from(dir).join(".claude.json")),
            None => Some(files::home()?.join(".claude.json")),
        },
    }
}

/// The folders people keep for more accounts beside `~/.claude`, such as
/// `~/.claude-2`, when they hold a login or settings. None with
/// `$CLAUDE_CONFIG_DIR` set: that folder alone is Claude Code's.
pub fn other_folders() -> Vec<PathBuf> {
    if std::env::var_os("CLAUDE_CONFIG_DIR").is_some() {
        return Vec::new();
    }
    let Some(entries) = files::home().and_then(|home| std::fs::read_dir(home).ok()) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".claude"))
        .map(|entry| entry.path())
        .filter(|dir| {
            !is_default(dir)
                && dir.is_dir()
                && (dir.join(".credentials.json").is_file() || dir.join(".claude.json").is_file())
        })
        .collect();
    dirs.sort();
    dirs
}

/// Whether the folder is Claude Code's default, `~/.claude`.
fn is_default(dir: &Path) -> bool {
    files::home().is_some_and(|home| dir == home.join(".claude"))
}

/// The account's name, from its folder: "2" for `~/.claude-2`, "work" for
/// `~/.claude-work`, none for `~/.claude`.
fn account(dir: &Path) -> Option<String> {
    let name = dir.file_name()?.to_string_lossy();
    let name = name.trim_start_matches('.');
    let rest = name
        .strip_prefix("claude")
        .unwrap_or(name)
        .trim_start_matches(['-', '_']);
    (!rest.is_empty()).then(|| rest.to_owned())
}

struct Login {
    token: String,
    /// Unix milliseconds.
    expires_at: i64,
    plan: Option<String>,
}

/// Claude Code's login: in the macOS keychain, read with the same `security`
/// command Claude Code uses, or in `~/.claude/.credentials.json` elsewhere.
/// Whichever expires last wins, since the file can be an old copy. The
/// keychain entry read is the default folder's, so another folder given
/// counts on its file alone.
fn login(given: Option<&Path>) -> Option<Login> {
    let from_file = config_dir(given)
        .and_then(|dir| std::fs::read_to_string(dir.join(".credentials.json")).ok())
        .and_then(|text| parse_login(&text));
    let from_keychain = given
        .is_none_or(is_default)
        .then(keychain)
        .flatten()
        .and_then(|text| parse_login(&text));
    match (from_keychain, from_file) {
        (Some(a), Some(b)) => Some(if b.expires_at > a.expires_at { b } else { a }),
        (a, b) => a.or(b),
    }
}

#[cfg(target_os = "macos")]
fn keychain() -> Option<String> {
    use std::process::{Command, Stdio};
    let user = std::env::var("USER").ok()?;
    let output = Command::new("/usr/bin/security")
        .args(["find-generic-password", "-a", &user, "-w", "-s"])
        .arg("Claude Code-credentials")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(not(target_os = "macos"))]
fn keychain() -> Option<String> {
    None
}

fn parse_login(text: &str) -> Option<Login> {
    let value: Value = serde_json::from_str(text.trim()).ok()?;
    let oauth = &value["claudeAiOauth"];
    let token = oauth["accessToken"]
        .as_str()
        .filter(|token| !token.is_empty())?;
    Some(Login {
        token: token.to_owned(),
        expires_at: oauth["expiresAt"].as_i64().unwrap_or(0),
        plan: plan(
            oauth["subscriptionType"].as_str(),
            oauth["rateLimitTier"].as_str(),
        ),
    })
}

/// "Max 20x" from the subscription "max" and the tier "default_claude_max_20x".
fn plan(subscription: Option<&str>, tier: Option<&str>) -> Option<String> {
    let mut name = super::plan_name(subscription?);
    if let Some(multiple) = tier
        .and_then(|tier| tier.rsplit('_').next())
        .filter(|last| last.ends_with('x') && last[..last.len() - 1].parse::<u32>().is_ok())
    {
        name = format!("{name} {multiple}");
    }
    Some(name)
}

/// The figures saved in Claude Code's settings, if they belong to the account
/// logged in there.
fn saved_usage(config: &Value) -> Option<Usage> {
    let cached = &config["cachedUsageUtilization"];
    let account = config["oauthAccount"]["accountUuid"].as_str();
    if let (Some(saved), Some(account)) = (cached["accountUuid"].as_str(), account)
        && saved != account
    {
        return None;
    }
    let fetched = cached["fetchedAtMs"].as_i64()? / 1000;
    parse(&cached["utilization"], Source::Saved, fetched, None)
}

/// The usage answer: `five_hour` and `seven_day`, plus the weekly limits of
/// single models once they count.
fn parse(body: &Value, source: Source, as_of: i64, plan: Option<String>) -> Option<Usage> {
    let mut windows: Vec<Window> = [("five_hour", Period::Session), ("seven_day", Period::Week)]
        .into_iter()
        .filter_map(|(key, period)| {
            let window = &body[key];
            Some(Window {
                period,
                used: (window["utilization"].as_f64()? / 100.0).clamp(0.0, 1.0),
                resets_at: window["resets_at"].as_str().and_then(parse_time),
                model: None,
            })
        })
        .collect();
    for limit in body["limits"].as_array().into_iter().flatten() {
        let percent = limit["percent"].as_f64().unwrap_or(0.0);
        if limit["kind"] != "weekly_scoped" || percent < 1.0 {
            continue;
        }
        windows.push(Window {
            period: Period::Week,
            used: (percent / 100.0).clamp(0.0, 1.0),
            resets_at: limit["resets_at"].as_str().and_then(parse_time),
            model: limit["scope"]["model"]["display_name"]
                .as_str()
                .map(str::to_owned),
        });
    }
    (!windows.is_empty()).then_some(Usage {
        tool: Tool::Claude,
        plan,
        windows,
        source,
        as_of,
        account: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &str = r#"{
        "five_hour": {"utilization": 57.0, "resets_at": "2026-09-25T20:09:59.699485+00:00"},
        "seven_day": {"utilization": 60.0, "resets_at": "2026-10-01T18:59:59+00:00"},
        "seven_day_opus": null,
        "limits": [
            {"kind": "session", "percent": 57, "resets_at": "2026-09-25T20:09:59+00:00"},
            {"kind": "weekly_scoped", "percent": 0, "scope": {"model": {"display_name": "Fable"}}},
            {"kind": "weekly_scoped", "percent": 12, "resets_at": "2026-10-01T18:59:59+00:00",
             "scope": {"model": {"display_name": "Opus"}}}
        ]
    }"#;

    #[test]
    fn reads_the_session_and_the_week() {
        let body: Value = serde_json::from_str(ANSWER).unwrap();
        let usage = parse(&body, Source::Live, 100, Some("Max 20x".into())).unwrap();
        assert_eq!(usage.windows.len(), 3, "a model at 0% is left out");
        let session = &usage.windows[0];
        assert_eq!(session.period, Period::Session);
        assert!((session.used - 0.57).abs() < 1e-9);
        assert_eq!(session.resets_at, Some(1_790_366_999));
        assert_eq!(usage.windows[1].period, Period::Week);
        assert_eq!(usage.windows[2].model.as_deref(), Some("Opus"));
        assert_eq!(usage.plan.as_deref(), Some("Max 20x"));
    }

    #[test]
    fn reads_the_figures_claude_code_saved() {
        let config = format!(
            r#"{{"oauthAccount": {{"accountUuid": "a"}},
                "cachedUsageUtilization": {{"fetchedAtMs": 1790361472000, "accountUuid": "a",
                                           "utilization": {ANSWER}}}}}"#
        );
        let config: Value = serde_json::from_str(&config).unwrap();
        let usage = saved_usage(&config).unwrap();
        assert_eq!((usage.source, usage.as_of), (Source::Saved, 1_790_361_472));
        let other = config
            .to_string()
            .replace(r#""accountUuid":"a"}"#, r#""accountUuid":"b"}"#);
        let other: Value = serde_json::from_str(&other).unwrap();
        assert_eq!(saved_usage(&other), None, "another account's figures");
        assert_eq!(saved_usage(&serde_json::json!({})), None);
    }

    #[test]
    fn reads_the_login_without_showing_it() {
        let login = parse_login(
            r#"{"claudeAiOauth": {"accessToken": "secret", "expiresAt": 1790000000000,
                "subscriptionType": "max", "rateLimitTier": "default_claude_max_20x"}}"#,
        )
        .unwrap();
        assert_eq!(login.expires_at, 1_790_000_000_000);
        assert_eq!(login.plan.as_deref(), Some("Max 20x"));
        assert!(parse_login(r#"{"claudeAiOauth": {"accessToken": ""}}"#).is_none());
        assert!(parse_login("not json").is_none());
        assert_eq!(
            plan(Some("pro"), Some("default_claude_ai")).as_deref(),
            Some("Pro")
        );
    }
}
