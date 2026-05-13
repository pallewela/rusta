mod common;
use std::io::Write;
use std::process::Stdio;

use common::{code, stderr, stdout, Harness};

#[test]
fn delete_without_yes_in_non_tty_errors() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    // No --yes and stdin is piped (not TTY): confirm() returns an error explaining
    // it needs --yes for non-interactive use.
    let out = h.run(&["delete", "lab"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("--yes") || stderr(&out).contains("TTY"));
    assert!(h.vm_state("lab").is_some(), "must not delete");
}

#[test]
fn create_with_ssh_copy_keys_chains_into_ssh_copy() {
    let h = Harness::new();
    h.write_dummy_ssh_key();
    let out = h.run(&["create", "lab", "--ssh-copy-keys"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("SSH configuration copied"));
}

#[test]
fn create_with_debug_no_headless_still_works() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--debug-no-headless"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
}

#[test]
fn delete_with_invalid_input_aborts_via_stdin_pipe() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    // Pipe an empty answer — confirm() treats anything not "y"/"yes" as no.
    // But confirm() needs a TTY, so stdin-piped triggers the non-TTY error path.
    let mut child = h.cmd(&["delete", "lab"])
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
    assert!(h.vm_state("lab").is_some());
}
