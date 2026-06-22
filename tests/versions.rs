mod common;

use common::{code, stdout, Harness, MockGhcr};

#[test]
fn versions_lists_and_marks_default() {
    let h = Harness::new();
    let mock = MockGhcr::start(
        r#"{"tags":["20.04","22.04","24.04","latest","99.x"]}"#,
        r#"{"token":"fake-token"}"#,
    );
    let mut cmd = h.cmd(&["versions"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", common::stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("20.04"));
    assert!(s.contains("22.04"));
    assert!(s.contains("24.04 (default)"));
    assert!(!s.contains("latest"));
    assert!(!s.contains("99.x"));
}

#[test]
fn versions_token_request_failure_is_fatal() {
    let h = Harness::new();
    let mut cmd = h.cmd(&["versions"]);
    // Point at an address nothing is listening on.
    cmd.env("RUSTA_GHCR_TOKEN_URL", "http://127.0.0.1:1/token");
    cmd.env("RUSTA_GHCR_TAGS_URL", "http://127.0.0.1:1/tags");
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
}

#[test]
fn versions_missing_token_field_is_fatal() {
    let h = Harness::new();
    let mock = MockGhcr::start(r#"{"tags":["24.04"]}"#, r#"{}"#);
    let mut cmd = h.cmd(&["versions"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
}

#[test]
fn versions_missing_tags_array_is_fatal() {
    let h = Harness::new();
    let mock = MockGhcr::start(r#"{}"#, r#"{"token":"t"}"#);
    let mut cmd = h.cmd(&["versions"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
}

#[test]
fn versions_multi_source_annotates_providers() {
    let h = Harness::new();
    // Adding a second source materializes the cirruslabs default too, so two
    // sources are queried and the output is annotated.
    assert_eq!(code(&h.run(&["source", "add", "ghcr.io/pallewela"])), 0);

    let mock = MockGhcr::start(r#"{"tags":["22.04","24.04"]}"#, r#"{"token":"fake-token"}"#);
    // Narrow to a single image so this exercises the single-image, multi-source
    // `from:` annotation (the default config has two images → matrix view).
    let mut cmd = h.cmd(&["versions", "--image", "ubuntu"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", common::stderr(&out));
    let s = stdout(&out);
    // Both sources report the same tags via the shared mock → both listed,
    // cirruslabs chosen on conflict (priority order).
    assert!(s.contains("from: cirruslabs, pallewela"), "out: {s}");
    assert!(s.contains("create uses cirruslabs"), "out: {s}");
    assert!(s.contains("24.04"), "out: {s}");
}
