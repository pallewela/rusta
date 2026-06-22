mod common;

use common::{code, stderr, stdout, Harness, MockGhcr};

#[test]
fn source_list_shows_seeded_default() {
    let h = Harness::new();
    let out = h.run(&["source"]); // no action → list
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("ghcr.io/cirruslabs"), "out: {s}");
    assert!(s.contains("built-in default"), "out: {s}");
}

#[test]
fn source_add_then_list_and_persist() {
    let h = Harness::new();
    assert_eq!(code(&h.run(&["source", "add", "ghcr.io/pallewela"])), 0);
    let s = stdout(&h.run(&["source", "list"]));
    assert!(s.contains("ghcr.io/cirruslabs"), "out: {s}");
    assert!(s.contains("ghcr.io/pallewela"), "out: {s}");
    let toml = std::fs::read_to_string(h.state_root.join("state.toml")).unwrap();
    assert!(toml.contains("ghcr.io/pallewela"), "state: {toml}");
}

#[test]
fn source_add_strips_trailing_ubuntu() {
    let h = Harness::new();
    assert_eq!(
        code(&h.run(&["source", "add", "ghcr.io/pallewela/ubuntu"])),
        0
    );
    let toml = std::fs::read_to_string(h.state_root.join("state.toml")).unwrap();
    assert!(
        toml.contains(r#"registry = "ghcr.io/pallewela""#),
        "state: {toml}"
    );
    assert!(!toml.contains("/ubuntu"), "should strip /ubuntu: {toml}");
}

#[test]
fn source_add_rejects_non_ghcr() {
    let h = Harness::new();
    let out = h.run(&["source", "add", "docker.io/library"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("ghcr.io"), "stderr: {}", stderr(&out));
}

#[test]
fn source_add_duplicate_is_skip() {
    let h = Harness::new();
    assert_eq!(code(&h.run(&["source", "add", "ghcr.io/pallewela"])), 0);
    let out = h.run(&["source", "add", "ghcr.io/pallewela"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("[skip]"), "out: {}", stdout(&out));
}

#[test]
fn source_rm_unknown_returns_2() {
    let h = Harness::new();
    let out = h.run(&["source", "rm", "ghcr.io/nope"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn source_move_reorders_priority() {
    let h = Harness::new();
    h.run(&["source", "add", "ghcr.io/pallewela"]); // [cirruslabs, pallewela]
    assert_eq!(
        code(&h.run(&["source", "move", "ghcr.io/pallewela", "1"])),
        0
    );
    let toml = std::fs::read_to_string(h.state_root.join("state.toml")).unwrap();
    let p = toml.find("pallewela").expect("pallewela present");
    let c = toml.find("cirruslabs").expect("cirruslabs present");
    assert!(p < c, "pallewela should now be first: {toml}");
}

#[test]
fn create_with_image_ref_clones_verbatim_offline() {
    let h = Harness::new();
    // --image-ref bypasses sources and registry queries entirely (no network).
    let out = h.run(&["create", "lab", "--image-ref", "ghcr.io/pallewela/ubuntu:22.04"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn create_with_unconfigured_source_errors() {
    let h = Harness::new();
    let out = h.run(&["create", "lab", "--source", "ghcr.io/ghost"]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("not configured"),
        "stderr: {}",
        stderr(&out)
    );
    assert!(h.vm_state("lab").is_none(), "must not create");
}

#[test]
fn create_multi_source_resolves_first_match() {
    let h = Harness::new();
    h.run(&["source", "add", "ghcr.io/pallewela"]); // [cirruslabs, pallewela]
    let mock = MockGhcr::start(r#"{"tags":["22.04","24.04"]}"#, r#"{"token":"t"}"#);
    let mut cmd = h.cmd(&["create", "lab", "--version", "22.04"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(h.vm_state("lab").as_deref(), Some("stopped"));
}

#[test]
fn create_multi_source_version_not_found_errors() {
    let h = Harness::new();
    h.run(&["source", "add", "ghcr.io/pallewela"]);
    let mock = MockGhcr::start(r#"{"tags":["24.04"]}"#, r#"{"token":"t"}"#);
    let mut cmd = h.cmd(&["create", "lab", "--version", "18.04"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("not found"),
        "stderr: {}",
        stderr(&out)
    );
    assert!(h.vm_state("lab").is_none(), "must not create");
}
