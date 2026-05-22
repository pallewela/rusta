mod common;
use common::{code, stdout, Harness};

#[test]
fn up_boots_stopped_vm() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["up", "lab"]);
    assert_eq!(code(&out), 0);
    assert_eq!(h.vm_state("lab").as_deref(), Some("running"));
}

#[test]
fn up_already_running_is_skip() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["up", "lab"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("[skip]"));
}

#[test]
fn up_unknown_vm_returns_2() {
    let h = Harness::new();
    let out = h.run(&["up", "ghost"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn up_with_graphical_flag_runs() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["up", "lab", "--graphical"]);
    assert_eq!(code(&out), 0);
    assert_eq!(h.vm_state("lab").as_deref(), Some("running"));
    let argv = h.last_run_args().expect("run logged");
    assert!(!argv.iter().any(|a| a == "--no-graphics"), "argv: {argv:?}");
}

#[test]
fn up_unknown_vm_defaults_to_graphical() {
    // No `[vms.lab]` entry in state.toml → default to graphical (issue #27).
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["up", "lab"]);
    assert_eq!(code(&out), 0);
    let argv = h.last_run_args().expect("run logged");
    assert!(!argv.iter().any(|a| a == "--no-graphics"), "argv: {argv:?}");
}

#[test]
fn up_after_create_with_gui_defaults_to_graphical() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--gui"]);
    assert_eq!(code(&out), 0);
    let out = h.run(&["up", "lab"]);
    assert_eq!(code(&out), 0);
    let argv = h.last_run_args().expect("run logged");
    assert!(!argv.iter().any(|a| a == "--no-graphics"), "argv: {argv:?}");
}

#[test]
fn up_after_create_without_gui_defaults_to_headless() {
    let h = Harness::new();
    let out = h.run(&["create", "lab"]);
    assert_eq!(code(&out), 0);
    let out = h.run(&["up", "lab"]);
    assert_eq!(code(&out), 0);
    let argv = h.last_run_args().expect("run logged");
    assert!(argv.iter().any(|a| a == "--no-graphics"), "argv: {argv:?}");
}

#[test]
fn up_no_gui_overrides_gui_default() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--gui"]);
    assert_eq!(code(&out), 0);
    let out = h.run(&["up", "lab", "--no-gui"]);
    assert_eq!(code(&out), 0);
    let argv = h.last_run_args().expect("run logged");
    assert!(argv.iter().any(|a| a == "--no-graphics"), "argv: {argv:?}");
}

#[test]
fn up_no_graphics_alias_overrides_gui_default() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--gui"]);
    assert_eq!(code(&out), 0);
    let out = h.run(&["up", "lab", "--no-graphics"]);
    assert_eq!(code(&out), 0);
    let argv = h.last_run_args().expect("run logged");
    assert!(argv.iter().any(|a| a == "--no-graphics"), "argv: {argv:?}");
}

#[test]
fn up_gui_alias_forces_graphical_on_headless_vm() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["up", "lab", "--gui"]);
    assert_eq!(code(&out), 0);
    let argv = h.last_run_args().expect("run logged");
    assert!(!argv.iter().any(|a| a == "--no-graphics"), "argv: {argv:?}");
}

#[test]
fn up_graphical_and_no_gui_conflict_exits_nonzero() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["up", "lab", "--graphical", "--no-gui"]);
    assert_ne!(code(&out), 0);
}

#[test]
fn down_graceful_stops_running_vm() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["down", "lab"]);
    assert_eq!(code(&out), 0);
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn down_already_stopped_is_skip() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let out = h.run(&["down", "lab"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("[skip]"));
}

#[test]
fn down_force_stops_immediately() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["down", "lab", "--force"]);
    assert_eq!(code(&out), 0);
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn ip_prints_address() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let out = h.run(&["ip", "lab"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("192.168.64.10"));
}

#[test]
fn ip_unknown_vm_returns_2() {
    let h = Harness::new();
    let out = h.run(&["ip", "ghost"]);
    assert_eq!(code(&out), 2);
}
