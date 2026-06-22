mod common;

use common::{code, stderr, stdout, Harness, MockGhcr};

#[test]
fn image_list_shows_seeded_defaults() {
    let h = Harness::new();
    let out = h.run(&["image"]); // no action → list
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let s = stdout(&out);
    // Both ubuntu and ubuntu-desktop are seeded by default; ubuntu is first.
    assert!(s.contains("ubuntu"), "out: {s}");
    assert!(s.contains("ubuntu-desktop"), "out: {s}");
    assert!(s.contains("(default)"), "out: {s}");
    assert!(s.contains("built-in default"), "out: {s}");
}

#[test]
fn image_add_then_list_and_persist() {
    let h = Harness::new();
    // ubuntu-kairos is not a seeded default, so adding it materializes the
    // defaults and appends.
    assert_eq!(code(&h.run(&["image", "add", "ubuntu-kairos"])), 0);
    let s = stdout(&h.run(&["image", "list"]));
    assert!(s.contains("ubuntu-kairos"), "out: {s}");
    let toml = std::fs::read_to_string(h.state_root.join("state.toml")).unwrap();
    assert!(toml.contains("ubuntu-kairos"), "state: {toml}");
    assert!(toml.contains("ubuntu-desktop"), "state: {toml}");
}

#[test]
fn image_add_seeded_default_is_skip() {
    let h = Harness::new();
    // ubuntu-desktop is already seeded by default → adding is a no-op skip.
    let out = h.run(&["image", "add", "ubuntu-desktop"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("[skip]"), "out: {}", stdout(&out));
}

#[test]
fn image_add_rejects_path_tag_and_uppercase() {
    let h = Harness::new();
    assert_eq!(code(&h.run(&["image", "add", "pallewela/ubuntu"])), 1);
    assert_eq!(code(&h.run(&["image", "add", "ubuntu:22.04"])), 1);
    assert_eq!(code(&h.run(&["image", "add", "Ubuntu"])), 1);
}

#[test]
fn image_add_duplicate_is_skip() {
    let h = Harness::new();
    assert_eq!(code(&h.run(&["image", "add", "ubuntu-kairos"])), 0);
    let out = h.run(&["image", "add", "ubuntu-kairos"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("[skip]"), "out: {}", stdout(&out));
}

#[test]
fn image_rm_unknown_returns_2() {
    let h = Harness::new();
    let out = h.run(&["image", "rm", "nope"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn image_rm_last_reseeds_default() {
    let h = Harness::new();
    // Remove both seeded defaults; the last removal re-seeds them (never imageless).
    assert_eq!(code(&h.run(&["image", "rm", "ubuntu-desktop"])), 0);
    assert_eq!(code(&h.run(&["image", "rm", "ubuntu"])), 0);
    let s = stdout(&h.run(&["image", "list"]));
    assert!(s.contains("ubuntu"), "out: {s}");
    assert!(s.contains("ubuntu-desktop"), "out: {s}");
}

#[test]
fn image_move_reorders_priority() {
    let h = Harness::new();
    // Defaults are [ubuntu, ubuntu-desktop]; move ubuntu-desktop ahead of ubuntu.
    assert_eq!(code(&h.run(&["image", "move", "ubuntu-desktop", "1"])), 0);
    let toml = std::fs::read_to_string(h.state_root.join("state.toml")).unwrap();
    assert!(
        toml.contains(r#"["ubuntu-desktop", "ubuntu"]"#),
        "state: {toml}"
    );
}

#[test]
fn create_with_image_clones_that_repo_offline() {
    let h = Harness::new();
    // Single (seeded) source → no network. The selected image is threaded into
    // the cloned reference verbatim.
    let out = h.run(&["create", "lab", "--image", "ubuntu-desktop"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("ghcr.io/cirruslabs/ubuntu-desktop:24.04"),
        "out: {}",
        stdout(&out)
    );
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn create_rejects_bad_image_name() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--image", "pallewela/ubuntu"]);
    assert_eq!(code(&out), 1);
    assert!(h.vm_state("lab").is_none(), "must not create");
}

#[test]
fn versions_matrix_lists_per_image() {
    let h = Harness::new();
    // Default config already has two images (ubuntu, ubuntu-desktop) → matrix.
    let mock = MockGhcr::start(r#"{"tags":["22.04","24.04"]}"#, r#"{"token":"t"}"#);
    let mut cmd = h.cmd(&["versions"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("ubuntu: "), "out: {s}");
    assert!(s.contains("ubuntu-desktop: "), "out: {s}");
}

#[test]
fn versions_image_filter_narrows_to_one() {
    let h = Harness::new();
    // ubuntu-desktop is a default image; narrow versions to just it.
    let mock = MockGhcr::start(r#"{"tags":["24.04"]}"#, r#"{"token":"t"}"#);
    let mut cmd = h.cmd(&["versions", "--image", "ubuntu-desktop"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    // Single image → legacy unannotated rendering (no "image:" segments).
    let s = stdout(&out);
    assert!(s.contains("24.04"), "out: {s}");
    assert!(!s.contains("ubuntu-desktop:"), "out: {s}");
}
