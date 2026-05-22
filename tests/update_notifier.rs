mod common;
use common::{stderr, Harness};

/// `Harness::run` defaults to `RUSTA_NO_UPDATE_CHECK=1`. Tests in this file
/// flip that off and inject the test seams: `RUSTA_UPDATE_PRETEND_TTY=1` to
/// bypass the real-TTY check (captured stderr is a pipe), and
/// `RUSTA_UPDATE_FORCE_LATEST=<version>` to fake the network response.
fn run_with_notifier(
    h: &Harness,
    args: &[&str],
    force_latest: Option<&str>,
    install_kind: &str,
) -> std::process::Output {
    let mut c = h.cmd(args);
    c.env_remove("RUSTA_NO_UPDATE_CHECK");
    c.env("RUSTA_UPDATE_PRETEND_TTY", "1");
    c.env("RUSTA_INSTALL_KIND", install_kind);
    if let Some(v) = force_latest {
        c.env("RUSTA_UPDATE_FORCE_LATEST", v);
    }
    c.output().expect("spawn rusta")
}

fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn bumped_patch() -> String {
    let v = current_version();
    let mut parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
    *parts.last_mut().unwrap() += 1;
    parts.iter().map(u64::to_string).collect::<Vec<_>>().join(".")
}

#[test]
fn notice_printed_when_newer_version_available() {
    let h = Harness::new();
    let newer = bumped_patch();
    let out = run_with_notifier(&h, &["list"], Some(&newer), "homebrew");
    let s = stderr(&out);
    assert!(s.contains("is available"), "stderr: {s}");
    assert!(s.contains(&newer), "stderr: {s}");
    assert!(s.contains(current_version()), "stderr: {s}");
}

#[test]
fn notice_suggests_brew_upgrade_for_homebrew_install() {
    let h = Harness::new();
    let newer = bumped_patch();
    let out = run_with_notifier(&h, &["list"], Some(&newer), "homebrew");
    assert!(stderr(&out).contains("brew upgrade rusta"), "stderr: {}", stderr(&out));
}

#[test]
fn notice_suggests_cargo_install_for_cargo_install() {
    let h = Harness::new();
    let newer = bumped_patch();
    let out = run_with_notifier(&h, &["list"], Some(&newer), "cargo");
    assert!(stderr(&out).contains("cargo install rusta"), "stderr: {}", stderr(&out));
}

#[test]
fn notice_falls_back_to_docs_link_for_unknown_install() {
    let h = Harness::new();
    let newer = bumped_patch();
    let out = run_with_notifier(&h, &["list"], Some(&newer), "other");
    let s = stderr(&out);
    assert!(s.contains("pallewela/rusta#installation"), "stderr: {s}");
}

#[test]
fn notice_includes_silence_hint() {
    let h = Harness::new();
    let newer = bumped_patch();
    let out = run_with_notifier(&h, &["list"], Some(&newer), "homebrew");
    assert!(
        stderr(&out).contains("RUSTA_NO_UPDATE_CHECK=1"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn notice_not_printed_when_up_to_date() {
    let h = Harness::new();
    // Force same version as current → no newer → no notice.
    let out = run_with_notifier(&h, &["list"], Some(current_version()), "homebrew");
    assert!(!stderr(&out).contains("is available"), "stderr: {}", stderr(&out));
}

#[test]
fn notice_not_printed_when_force_latest_empty() {
    let h = Harness::new();
    // Empty string is the "up to date / no newer version" signal.
    let out = run_with_notifier(&h, &["list"], Some(""), "homebrew");
    assert!(!stderr(&out).contains("is available"), "stderr: {}", stderr(&out));
}

#[test]
fn notice_not_printed_when_no_update_check_env_set() {
    let h = Harness::new();
    // Default harness behavior — RUSTA_NO_UPDATE_CHECK=1 is set.
    // Even with a forced "newer" version, the spawn is short-circuited.
    let mut c = h.cmd(&["list"]);
    c.env("RUSTA_UPDATE_PRETEND_TTY", "1");
    c.env("RUSTA_UPDATE_FORCE_LATEST", "999.0.0");
    c.env("RUSTA_INSTALL_KIND", "homebrew");
    let out = c.output().expect("spawn rusta");
    assert!(!stderr(&out).contains("is available"), "stderr: {}", stderr(&out));
}

#[test]
fn notice_not_printed_when_stderr_is_not_tty() {
    let h = Harness::new();
    // Same as the success case but without PRETEND_TTY. Captured stderr is
    // a pipe → is_terminal() returns false → spawn short-circuits.
    let mut c = h.cmd(&["list"]);
    c.env_remove("RUSTA_NO_UPDATE_CHECK");
    c.env("RUSTA_UPDATE_FORCE_LATEST", "999.0.0");
    c.env("RUSTA_INSTALL_KIND", "homebrew");
    let out = c.output().expect("spawn rusta");
    assert!(!stderr(&out).contains("is available"), "stderr: {}", stderr(&out));
}

#[test]
fn notice_throttled_on_second_run_within_24h() {
    let h = Harness::new();
    let newer = bumped_patch();
    let first = run_with_notifier(&h, &["list"], Some(&newer), "homebrew");
    assert!(stderr(&first).contains("is available"), "first stderr: {}", stderr(&first));
    let second = run_with_notifier(&h, &["list"], Some(&newer), "homebrew");
    assert!(
        !stderr(&second).contains("is available"),
        "second invocation should be throttled, stderr: {}",
        stderr(&second)
    );
}

#[test]
fn notice_not_printed_when_latest_is_prerelease_and_current_is_stable() {
    let h = Harness::new();
    // current is stable (CARGO_PKG_VERSION has no "-"). Latest is a
    // pre-release of a newer major. Per the channel-matching rule, suppress.
    let out = run_with_notifier(&h, &["list"], Some("99.0.0-rc.1"), "homebrew");
    assert!(!stderr(&out).contains("is available"), "stderr: {}", stderr(&out));
}

#[test]
fn state_file_records_check_and_notification_timestamps() {
    let h = Harness::new();
    let newer = bumped_patch();
    let _ = run_with_notifier(&h, &["list"], Some(&newer), "homebrew");
    let s = std::fs::read_to_string(h.state_root.join("state.toml")).unwrap();
    assert!(s.contains("[update]"), "state.toml: {s}");
    assert!(s.contains("last_checked_at"), "state.toml: {s}");
    assert!(s.contains("last_notified_at"), "state.toml: {s}");
    assert!(s.contains(&format!("latest_known = \"{newer}\"")), "state.toml: {s}");
}
