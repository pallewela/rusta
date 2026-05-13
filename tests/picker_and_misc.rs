mod common;
use common::{code, stderr, stdout, Harness};

#[test]
fn picker_no_default_no_vms_returns_2() {
    let h = Harness::new();
    let out = h.run(&["up"]);
    assert_eq!(code(&out), 2);
    assert!(stderr(&out).contains("Create one"));
}

#[test]
fn picker_no_default_non_tty_returns_2() {
    let h = Harness::new();
    h.add_vm("a", "stopped");
    h.add_vm("b", "stopped");
    let out = h.run(&["up"]);
    // stdin in `cargo test` subprocess is piped → not a TTY → picker bails with 2.
    assert_eq!(code(&out), 2);
    assert!(stderr(&out).contains("not a TTY"));
}

#[test]
fn picker_uses_existing_default() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let _ = h.run(&["default", "lab"]);
    let out = h.run(&["up"]); // no explicit name
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(h.vm_state("lab").as_deref(), Some("running"));
}

#[test]
fn picker_default_is_stale_falls_back_and_errors_non_tty() {
    let h = Harness::new();
    // Pretend default was set to a VM that no longer exists.
    let state = h.state_root.join("state.toml");
    std::fs::create_dir_all(&h.state_root).unwrap();
    std::fs::write(&state, "default_vm = \"ghost\"\n").unwrap();
    h.add_vm("real", "stopped");
    let out = h.run(&["ip"]); // no arg; default stale; non-TTY → exit 2
    assert_eq!(code(&out), 2);
    let combined = format!("{}{}", stdout(&out), stderr(&out));
    assert!(combined.contains("no longer exists") || combined.contains("not a TTY"));
}

#[test]
fn verbose_global_flag_does_not_error() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["--verbose", "list"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("lab"));
}

#[test]
fn log_flag_tees_output_to_file() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let log = h.root.join("session.log");
    let out = h.run(&["--log", log.to_str().unwrap(), "list"]);
    assert_eq!(code(&out), 0);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(contents.contains("lab"));
    assert!(contents.contains("Logging all output to"));
}

#[test]
fn down_graceful_timeout_fails_with_retry_hint() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    // Sabotage the fake tart so `exec` (used for shutdown) fails and state stays running.
    // We do this by pointing RUSTA_TART_BIN at a script that ignores `exec` arguments.
    let sabotage = h.root.join("bin/tart-no-shutdown");
    std::fs::write(
        &sabotage,
        r##"#!/usr/bin/env bash
case "${1:-}" in
  list) "$RUSTA_REAL_TART" "$@" ;;
  exec) exit 1 ;;        # always fail exec → shutdown request never reaches our state
  ip) echo "192.168.64.10" ;;
  *) "$RUSTA_REAL_TART" "$@" ;;
esac
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(&sabotage).unwrap().permissions();
    p.set_mode(0o755);
    std::fs::set_permissions(&sabotage, p).unwrap();

    let real_tart = h.bin_dir.join("fake-tart");
    let mut cmd = h.cmd(&["down", "lab", "--timeout", "1"]);
    cmd.env("RUSTA_TART_BIN", &sabotage);
    cmd.env("RUSTA_REAL_TART", &real_tart);
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("--force"));
}
