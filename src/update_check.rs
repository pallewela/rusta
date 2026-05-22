//! Background update notifier.
//!
//! Spawned at the start of `main()` (after argument parsing) and joined
//! with a short timeout just before exit, so a slow GitHub round-trip
//! never adds latency to the user's command. Prints a one-line notice to
//! stderr when a newer release exists, at most once per 24h, only when
//! stderr is a TTY, and never when `RUSTA_NO_UPDATE_CHECK` is set.

use std::io::IsTerminal;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::state::State;

const RELEASES_URL: &str = "https://api.github.com/repos/pallewela/rusta/releases/latest";
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const JOIN_TIMEOUT: Duration = Duration::from_millis(100);
const NOTIFY_INTERVAL_SECS: u64 = 24 * 60 * 60;
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

pub fn maybe_spawn() -> Option<JoinHandle<Option<String>>> {
    if std::env::var_os("RUSTA_NO_UPDATE_CHECK").is_some() {
        return None;
    }
    if !stderr_is_tty() {
        return None;
    }
    Some(std::thread::spawn(fetch_latest_or_cached))
}

pub fn maybe_finalize(handle: Option<JoinHandle<Option<String>>>) {
    let Some(handle) = handle else { return };
    let start = Instant::now();
    let latest = loop {
        if handle.is_finished() {
            break handle.join().ok().flatten();
        }
        if start.elapsed() >= JOIN_TIMEOUT {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let Some(latest) = latest else { return };
    notify_if_due(&latest);
}

fn stderr_is_tty() -> bool {
    // Test seam: setting RUSTA_UPDATE_PRETEND_TTY lets integration tests
    // exercise the notify path even though their captured stderr is a pipe.
    if std::env::var_os("RUSTA_UPDATE_PRETEND_TTY").is_some() {
        return true;
    }
    std::io::stderr().is_terminal()
}

fn fetch_latest_or_cached() -> Option<String> {
    let s = State::load();
    let now = unix_now();
    if let Some(u) = s.update.as_ref() {
        if now.saturating_sub(u.last_checked_at) < CHECK_INTERVAL_SECS {
            return u.latest_known.clone();
        }
    }
    let latest = fetch_remote().ok().flatten()?;
    record_check(&latest);
    Some(latest)
}

fn fetch_remote() -> Result<Option<String>, String> {
    // Test seam: RUSTA_UPDATE_FORCE_LATEST short-circuits the network call.
    // Empty string means "up to date" (no newer version); any other value is
    // treated as the latest version string from the registry.
    if let Ok(forced) = std::env::var("RUSTA_UPDATE_FORCE_LATEST") {
        return Ok(if forced.is_empty() { None } else { Some(forced) });
    }
    let url = std::env::var("RUSTA_UPDATE_URL").unwrap_or_else(|_| RELEASES_URL.to_string());
    let agent = ureq::AgentBuilder::new().timeout(HTTP_TIMEOUT).build();
    let resp: Value = agent
        .get(&url)
        .set(
            "User-Agent",
            &format!("rusta/{}", env!("CARGO_PKG_VERSION")),
        )
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;
    let tag = resp
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing tag_name".to_string())?;
    Ok(Some(tag.trim_start_matches('v').to_string()))
}

fn record_check(latest: &str) {
    let mut s = State::load();
    let mut u = s.update.clone().unwrap_or_default();
    u.last_checked_at = unix_now();
    u.latest_known = Some(latest.to_string());
    s.update = Some(u);
    let _ = s.save();
}

fn notify_if_due(latest: &str) {
    let current = env!("CARGO_PKG_VERSION");
    if !is_newer(latest, current) {
        return;
    }
    if !channel_matches(latest, current) {
        return;
    }
    let mut s = State::load();
    let mut u = s.update.clone().unwrap_or_default();
    let now = unix_now();
    if now.saturating_sub(u.last_notified_at) < NOTIFY_INTERVAL_SECS {
        return;
    }
    print_notice(latest, current);
    u.last_notified_at = now;
    s.update = Some(u);
    let _ = s.save();
}

fn print_notice(latest: &str, current: &str) {
    let cmd = upgrade_command();
    let (bold_green, dim, reset) = if color_enabled() {
        ("\x1b[1;32m", "\x1b[2m", "\x1b[0m")
    } else {
        ("", "", "")
    };
    eprintln!();
    eprintln!("  rusta {bold_green}{latest}{reset} is available (you have {current}). {cmd}");
    eprintln!("  {dim}Silence: RUSTA_NO_UPDATE_CHECK=1{reset}");
}

/// Color is enabled when stderr is a TTY *and* `NO_COLOR` is unset *and*
/// `TERM != dumb`. The `RUSTA_UPDATE_PRETEND_TTY` test seam from #32
/// flows through so integration tests can exercise the color path even
/// though their captured stderr is a pipe.
fn color_enabled() -> bool {
    let tty = std::env::var_os("RUSTA_UPDATE_PRETEND_TTY").is_some()
        || std::io::stderr().is_terminal();
    if !tty {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("TERM").as_deref() == Ok("dumb") {
        return false;
    }
    true
}

fn upgrade_command() -> String {
    match detect_install() {
        InstallKind::Homebrew => "Run `brew upgrade rusta` to update.".to_string(),
        InstallKind::Cargo => "Run `cargo install rusta` to update.".to_string(),
        InstallKind::Other => {
            "See https://github.com/pallewela/rusta#installation".to_string()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstallKind {
    Homebrew,
    Cargo,
    Other,
}

pub fn detect_install() -> InstallKind {
    if let Ok(forced) = std::env::var("RUSTA_INSTALL_KIND") {
        return match forced.as_str() {
            "homebrew" => InstallKind::Homebrew,
            "cargo" => InstallKind::Cargo,
            _ => InstallKind::Other,
        };
    }
    let exe = std::env::current_exe()
        .and_then(|p| std::fs::canonicalize(&p).or(Ok(p)))
        .ok();
    let Some(path) = exe else {
        return InstallKind::Other;
    };
    let s = path.to_string_lossy();
    if s.contains("/Cellar/rusta/") || s.contains("/Cellar/rusta-cli/") {
        InstallKind::Homebrew
    } else if s.contains("/.cargo/bin/") || s.contains("/cargo/bin/") {
        InstallKind::Cargo
    } else {
        InstallKind::Other
    }
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Pre-release filter: if the running version is stable, suppress notices
/// about pre-release versions. Otherwise allow.
pub fn channel_matches(latest: &str, current: &str) -> bool {
    !is_prerelease(latest) || is_prerelease(current)
}

fn is_prerelease(v: &str) -> bool {
    let v = v.trim_start_matches('v');
    let v = v.split('+').next().unwrap_or(v);
    v.contains('-')
}

/// Semver core sort tuple. Pre-release versions sort *before* the same
/// numeric core (per semver §11), which we model by mapping `None` → high
/// sentinel so stable > pre.
fn parse_semver(v: &str) -> Option<(u64, u64, u64, u8, String)> {
    let v = v.trim_start_matches('v');
    let v = v.split('+').next().unwrap_or(v);
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, p.to_string()),
        None => (v, String::new()),
    };
    let mut parts = core.split('.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    let patch: u64 = parts.next()?.parse().ok()?;
    // pre_rank: 1 = pre-release present, 0 = absent; bigger sorts later, so
    // empty pre (stable) gets 1 and a pre-release gets 0.
    let pre_rank = if pre.is_empty() { 1 } else { 0 };
    Some((major, minor, patch, pre_rank, pre))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_basic() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("1.1.0", "1.0.99"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_newer_strips_v_prefix() {
        assert!(is_newer("v1.0.1", "1.0.0"));
        assert!(is_newer("1.0.1", "v1.0.0"));
    }

    #[test]
    fn is_newer_handles_build_metadata() {
        // build metadata must be ignored
        assert!(!is_newer("1.0.0+sha.abc", "1.0.0"));
        assert!(is_newer("1.0.1+sha.abc", "1.0.0"));
    }

    #[test]
    fn is_newer_prerelease_ordering() {
        // Stable beats matching pre-release (semver §11).
        assert!(is_newer("1.0.0", "1.0.0-beta.1"));
        assert!(!is_newer("1.0.0-beta.1", "1.0.0"));
        // Pre-release vs pre-release: lexical on the pre part is good enough
        // for our purposes since we only call this for notification gating.
        assert!(is_newer("1.0.0-beta.2", "1.0.0-beta.1"));
    }

    #[test]
    fn channel_suppresses_pre_for_stable_users() {
        // Stable user must not be notified of a pre-release.
        assert!(!channel_matches("2.0.0-rc.1", "1.0.0"));
        // Pre-release user is notified of any newer pre-release.
        assert!(channel_matches("2.0.0-rc.2", "2.0.0-rc.1"));
        // Pre-release user is notified of a stable release.
        assert!(channel_matches("2.0.0", "2.0.0-rc.1"));
        // Stable user notified of stable.
        assert!(channel_matches("1.0.1", "1.0.0"));
    }

    #[test]
    fn detect_install_respects_env_override() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev = std::env::var_os("RUSTA_INSTALL_KIND");
        std::env::set_var("RUSTA_INSTALL_KIND", "homebrew");
        assert_eq!(detect_install(), InstallKind::Homebrew);
        std::env::set_var("RUSTA_INSTALL_KIND", "cargo");
        assert_eq!(detect_install(), InstallKind::Cargo);
        std::env::set_var("RUSTA_INSTALL_KIND", "other");
        assert_eq!(detect_install(), InstallKind::Other);
        match prev {
            Some(v) => std::env::set_var("RUSTA_INSTALL_KIND", v),
            None => std::env::remove_var("RUSTA_INSTALL_KIND"),
        }
    }

    #[test]
    fn parse_semver_rejects_garbage() {
        assert!(parse_semver("not-a-version").is_none());
        assert!(parse_semver("1.0").is_none());
        assert!(parse_semver("").is_none());
    }

    #[test]
    fn is_prerelease_matrix() {
        assert!(is_prerelease("1.0.0-beta.1"));
        assert!(is_prerelease("v2.0.0-rc.2"));
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("v1.0.0"));
        // Build metadata is not pre-release.
        assert!(!is_prerelease("1.0.0+sha.abc"));
    }

    #[test]
    fn color_enabled_decision_matrix() {
        let _g = ENV_LOCK.lock().unwrap();
        // Snapshot the four env vars we touch, restore on drop.
        let prev_pretend = std::env::var_os("RUSTA_UPDATE_PRETEND_TTY");
        let prev_no_color = std::env::var_os("NO_COLOR");
        let prev_term = std::env::var_os("TERM");
        // Clear them all so the baseline is deterministic.
        std::env::remove_var("RUSTA_UPDATE_PRETEND_TTY");
        std::env::remove_var("NO_COLOR");
        std::env::remove_var("TERM");

        // Baseline: stderr is almost certainly a pipe under `cargo test` —
        // without PRETEND_TTY we expect color_enabled() == false.
        assert!(!color_enabled(), "no TTY, no PRETEND_TTY → no color");

        // PRETEND_TTY alone → color on.
        std::env::set_var("RUSTA_UPDATE_PRETEND_TTY", "1");
        assert!(color_enabled(), "PRETEND_TTY=1 → color on");

        // NO_COLOR set to *anything* (including empty) → color off.
        std::env::set_var("NO_COLOR", "1");
        assert!(!color_enabled(), "NO_COLOR=1 → color off");
        std::env::set_var("NO_COLOR", "");
        assert!(!color_enabled(), "NO_COLOR='' (set but empty) still → color off");
        std::env::remove_var("NO_COLOR");

        // TERM=dumb → color off.
        std::env::set_var("TERM", "dumb");
        assert!(!color_enabled(), "TERM=dumb → color off");
        // Other TERM values are fine.
        std::env::set_var("TERM", "xterm-256color");
        assert!(color_enabled(), "TERM=xterm-256color → color on");

        // Restore.
        match prev_pretend {
            Some(v) => std::env::set_var("RUSTA_UPDATE_PRETEND_TTY", v),
            None => std::env::remove_var("RUSTA_UPDATE_PRETEND_TTY"),
        }
        match prev_no_color {
            Some(v) => std::env::set_var("NO_COLOR", v),
            None => std::env::remove_var("NO_COLOR"),
        }
        match prev_term {
            Some(v) => std::env::set_var("TERM", v),
            None => std::env::remove_var("TERM"),
        }
    }

    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());
}
