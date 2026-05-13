mod common;
use common::{code, stderr, stdout, Harness};

#[test]
fn create_defaults_to_ubuntu_2404() {
    let h = Harness::new();
    let out = h.run(&["create"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(h.vm_state("ubuntu-2404").as_deref(), Some("stopped"));
    // Provisioning script was generated.
    assert!(h
        .state_root
        .join("provision/ubuntu-2404.sh")
        .exists());
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
    let _ = h.run(&["create"]);
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
