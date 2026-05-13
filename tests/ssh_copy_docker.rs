mod common;
use common::{code, stderr, stdout, Harness};

#[test]
fn ssh_unknown_vm_returns_2() {
    let h = Harness::new();
    let out = h.run(&["ssh", "ghost"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn ssh_running_vm_passes_through_fake_ssh() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["ssh", "lab"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
}

#[test]
fn ssh_not_running_without_auto_up_fails() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["ssh", "lab"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("not running"));
}

#[test]
fn ssh_auto_up_boots_then_connects() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["ssh", "lab", "--auto-up"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(h.vm_state("lab").as_deref(), Some("running"));
}

#[test]
fn ssh_with_remote_command() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["ssh", "lab", "--", "uname", "-a"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
}

#[test]
fn ssh_copy_with_keys_succeeds() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    h.write_dummy_ssh_key();
    h.write_pem("cert.pem");
    let out = h.run(&["ssh-copy", "lab"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("SSH configuration copied"));
    // Started by us → should be stopped at the end.
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn ssh_copy_when_no_keys_is_skip() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["ssh-copy", "lab"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("[skip]"));
}

#[test]
fn ssh_copy_unknown_vm_returns_2() {
    let h = Harness::new();
    let out = h.run(&["ssh-copy", "ghost"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn docker_setup_writes_ssh_config_and_creates_context() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    // Pre-create the SSH key pair so docker_setup doesn't shell out to ssh-keygen
    // (we want to assert the path is exercised, but ssh-keygen as `true` won't
    // create files — pre-creating is the simplest deterministic approach).
    h.write_dummy_ssh_key();
    let out = h.run(&["docker-setup", "lab"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let cfg = std::fs::read_to_string(h.ssh_dir.join("config")).unwrap();
    assert!(cfg.contains("Host docker-lab"));
    assert!(cfg.contains("HostName 192.168.64.10"));
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn docker_setup_idempotent_on_rerun() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    h.write_dummy_ssh_key();
    let out1 = h.run(&["docker-setup", "lab"]);
    assert_eq!(code(&out1), 0);
    let out2 = h.run(&["docker-setup", "lab"]);
    assert_eq!(code(&out2), 0);
    assert!(stdout(&out2).contains("already exists"));
}

#[test]
fn docker_setup_when_vm_already_running_does_not_shutdown() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    h.write_dummy_ssh_key();
    let out = h.run(&["docker-setup", "lab"]);
    assert_eq!(code(&out), 0);
    // Started by user → rusta should leave it running.
    assert_eq!(h.vm_state("lab").as_deref(), Some("running"));
}

#[test]
fn docker_setup_unknown_vm_returns_2() {
    let h = Harness::new();
    let out = h.run(&["docker-setup", "ghost"]);
    assert_eq!(code(&out), 2);
}
