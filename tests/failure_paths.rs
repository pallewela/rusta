mod common;
use common::{code, stderr, write_sshpass_probe_ok_else_fail, Harness};

#[test]
fn ssh_copy_fails_when_sshpass_returns_error() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    h.write_dummy_ssh_key();
    let sshpass = h.root.join("bin/sshpass-probe-ok");
    write_sshpass_probe_ok_else_fail(&sshpass);
    let mut cmd = h.cmd(&["ssh-copy", "lab"]);
    cmd.env("RUSTA_SSHPASS_BIN", &sshpass);
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("ssh"));
}

#[test]
fn ssh_interactive_propagates_exit_code() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    let sshpass = h.root.join("bin/sshpass-probe-ok");
    write_sshpass_probe_ok_else_fail(&sshpass);
    let mut cmd = h.cmd(&["ssh", "lab"]);
    cmd.env("RUSTA_SSHPASS_BIN", &sshpass);
    let out = cmd.output().unwrap();
    // ssh_interactive returns the child exit code as the rusta exit code.
    assert_ne!(code(&out), 0);
}

#[test]
fn tart_list_failure_surfaces_error() {
    let h = Harness::new();
    let mut cmd = h.cmd(&["list"]);
    // Point at a non-existent binary → Command::new(...) spawn fails.
    cmd.env("RUSTA_TART_BIN", "/definitely/not/a/binary");
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("tart"));
}

#[test]
fn tart_ip_failure_surfaces_error() {
    let h = Harness::new();
    h.add_vm("lab", "running");
    // Build a fake tart that succeeds for `list` (so existence check passes)
    // but fails for `ip`.
    let real = h.bin_dir.join("fake-tart");
    let sabotage = h.root.join("bin/tart-ip-fail");
    std::fs::write(
        &sabotage,
        r##"#!/usr/bin/env bash
case "${1:-}" in
  ip) exit 1 ;;
  *) exec "$RUSTA_REAL_TART" "$@" ;;
esac
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(&sabotage).unwrap().permissions();
    p.set_mode(0o755);
    std::fs::set_permissions(&sabotage, p).unwrap();

    let mut cmd = h.cmd(&["ip", "lab"]);
    cmd.env("RUSTA_TART_BIN", &sabotage);
    cmd.env("RUSTA_REAL_TART", &real);
    cmd.env("RUSTA_POLL_MS", "5");
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
}

#[test]
fn create_fails_when_tart_clone_fails() {
    let h = Harness::new();
    let sabotage = h.root.join("bin/tart-clone-fail");
    let real = h.bin_dir.join("fake-tart");
    std::fs::write(
        &sabotage,
        r##"#!/usr/bin/env bash
case "${1:-}" in
  clone) exit 1 ;;
  *) exec "$RUSTA_REAL_TART" "$@" ;;
esac
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(&sabotage).unwrap().permissions();
    p.set_mode(0o755);
    std::fs::set_permissions(&sabotage, p).unwrap();

    let mut cmd = h.cmd(&["create", "lab"]);
    cmd.env("RUSTA_TART_BIN", &sabotage);
    cmd.env("RUSTA_REAL_TART", &real);
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("tart clone"));
}

#[test]
fn docker_setup_fails_when_docker_context_create_fails() {
    let h = Harness::new();
    h.add_vm("lab", "stopped");
    h.write_dummy_ssh_key();
    let mut cmd = h.cmd(&["docker-setup", "lab"]);
    // Use plain `false` for docker → context inspect fails (good) AND context create fails.
    cmd.env("RUSTA_DOCKER_BIN", "false");
    // `which("docker")` runs `command -v false` which succeeds (false is a real bin),
    // so ensure_docker_cli passes and the failure surfaces from `docker context create`.
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("docker context"));
}
