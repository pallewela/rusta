mod common;
use std::os::unix::fs::PermissionsExt;

use common::{code, stderr, Harness};

fn write_script(path: &std::path::Path, body: &str) {
    std::fs::write(path, body).unwrap();
    let mut p = std::fs::metadata(path).unwrap().permissions();
    p.set_mode(0o755);
    std::fs::set_permissions(path, p).unwrap();
}

#[test]
fn preflight_pass_on_arm64_with_tart_present() {
    let h = Harness::new();
    let uname = h.root.join("bin/uname-arm");
    write_script(&uname, "#!/usr/bin/env bash\necho arm64\n");
    let mut cmd = h.cmd(&["list"]);
    cmd.env_remove("RUSTA_SKIP_PREFLIGHT");
    cmd.env("RUSTA_UNAME_BIN", &uname);
    // brew = true (which() succeeds), tart already redirected to fake-tart (which() succeeds).
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
}

#[test]
fn preflight_rejects_non_arm() {
    let h = Harness::new();
    let uname = h.root.join("bin/uname-x86");
    write_script(&uname, "#!/usr/bin/env bash\necho x86_64\n");
    let mut cmd = h.cmd(&["list"]);
    cmd.env_remove("RUSTA_SKIP_PREFLIGHT");
    cmd.env("RUSTA_UNAME_BIN", &uname);
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("Apple Silicon"));
}

#[test]
fn preflight_rejects_missing_brew() {
    let h = Harness::new();
    let uname = h.root.join("bin/uname-arm");
    write_script(&uname, "#!/usr/bin/env bash\necho arm64\n");
    let mut cmd = h.cmd(&["list"]);
    cmd.env_remove("RUSTA_SKIP_PREFLIGHT");
    cmd.env("RUSTA_UNAME_BIN", &uname);
    cmd.env("RUSTA_BREW_BIN", "/definitely/not/a/binary");
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("Homebrew"));
}

#[test]
fn preflight_installs_tart_when_missing() {
    let h = Harness::new();
    let uname = h.root.join("bin/uname-arm");
    write_script(&uname, "#!/usr/bin/env bash\necho arm64\n");
    // Make `which("tart")` fail by pointing tart at a non-existent path...
    // but ALSO have the `brew install` call (via `true`) succeed.
    let mut cmd = h.cmd(&["list"]);
    cmd.env_remove("RUSTA_SKIP_PREFLIGHT");
    cmd.env("RUSTA_UNAME_BIN", &uname);
    cmd.env("RUSTA_TART_BIN", "/definitely/not/a/binary");
    let out = cmd.output().unwrap();
    // After brew "install" succeeds (true), rusta proceeds to call `tart list`
    // which fails because the bin still doesn't exist. We just check it
    // attempted the install path (no Homebrew error).
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("tart") && !stderr(&out).contains("Homebrew"));
}
