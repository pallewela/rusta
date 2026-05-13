mod common;
use common::{code, stdout, Harness};

#[test]
fn list_empty_succeeds() {
    let h = Harness::new();
    let out = h.run(&["list"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("NAME"));
}

#[test]
fn list_shows_added_vms() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    h.add_vm("alpha", "running");
    let out = h.run(&["list"]);
    let s = stdout(&out);
    assert!(s.contains("lab"));
    assert!(s.contains("alpha"));
    assert!(s.contains("running"));
}

#[test]
fn default_with_no_state_prints_message_and_exits_one() {
    let h = Harness::new();
    let out = h.run(&["default"]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).contains("no default set"));
}

#[test]
fn default_set_then_get() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    let set = h.run(&["default", "lab"]);
    assert_eq!(code(&set), 0);
    let get = h.run(&["default"]);
    assert_eq!(code(&get), 0);
    assert!(stdout(&get).contains("lab"));
}

#[test]
fn default_set_unknown_vm_returns_2() {
    let h = Harness::new();
    let out = h.run(&["default", "nope"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn list_marks_default_with_asterisk() {
    let h = Harness::new();
    h.add_vm("a", "stopped");
    h.add_vm("b", "stopped");
    let _ = h.run(&["default", "b"]);
    let out = h.run(&["list"]);
    let s = stdout(&out);
    // The 'b' row should have a trailing '*'. We just check that '*' appears.
    assert!(s.contains('*'));
}
