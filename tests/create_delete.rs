mod common;
use std::io::Write;
use std::process::Stdio;

use common::{code, stderr, stdout, Harness};

#[test]
fn create_without_name_in_non_tty_errors() {
    let h = Harness::new();
    // Test subprocess stdin is a pipe (not a TTY): create must refuse to
    // synthesize a name and exit 1 with a helpful message.
    let out = h.run(&["create"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("VM name is required"));
    assert!(h.vm_state("ubuntu-2404").is_none(), "must not create");
}

#[test]
fn create_with_explicit_name_creates_it() {
    let h = Harness::new();
    let out = h.run(&["create", "ubuntu-2404"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(h.vm_state("ubuntu-2404").as_deref(), Some("stopped"));
    assert!(h.state_root.join("provision/ubuntu-2404.sh").exists());
}

#[test]
fn create_without_name_aborts_on_eof_via_stdin_pipe() {
    // Even when stdin is piped (so the non-TTY guard fires first), we should
    // not synthesize a name silently. The TTY branch is exercised by the
    // unit tests for `prompt_for_name`.
    let h = Harness::new();
    let mut child = h
        .cmd(&["create"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(mut s) = child.stdin.take() {
        let _ = s.write_all(b"\n");
    }
    let out = child.wait_with_output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(h.vm_state("ubuntu-2404").is_none());
}

#[test]
fn create_with_explicit_name_and_version() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--version", "22.04"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn create_existing_vm_is_skip() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["create", "lab"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("[skip]"));
    assert!(stdout(&out).contains("To recreate"));
}

#[test]
fn create_with_gui_works() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--gui", "ubuntu-desktop"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let script = std::fs::read_to_string(h.state_root.join("provision/lab.sh")).unwrap();
    assert!(script.contains("ubuntu-desktop gdm3"));
}

#[test]
fn create_with_invalid_gui_errors() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--gui", "kde-desktop"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("--gui accepts"));
}

#[test]
fn create_with_invalid_name_errors() {
    let h = Harness::new();
    let out = h.run(&["create", "has space"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("invalid VM name"));
}

#[test]
fn create_does_not_set_default() {
    let h = Harness::new();
    let _ = h.run(&["create", "lab"]);
    let out = h.run(&["default"]);
    assert_eq!(code(&out), 1);
}

#[test]
fn delete_with_yes_skips_prompt_and_clears_default() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let _ = h.run(&["default", "lab"]);
    let out = h.run(&["delete", "lab", "--yes"]);
    assert_eq!(code(&out), 0);
    assert!(h.vm_state("lab").is_none());
    let def = h.run(&["default"]);
    assert_eq!(code(&def), 1);
}

#[test]
fn delete_unknown_vm_returns_2() {
    let h = Harness::new();
    let out = h.run(&["delete", "ghost", "--yes"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn delete_refuses_running_vm_without_force() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["delete", "lab", "--yes"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("running"));
    assert!(h.vm_state("lab").is_some(), "must not delete");
}

#[test]
fn delete_force_running_stops_and_deletes() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["delete", "lab", "--yes", "--force-running"]);
    assert_eq!(code(&out), 0);
    assert!(h.vm_state("lab").is_none());
}
