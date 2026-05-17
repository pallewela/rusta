mod common;
use common::{code, stderr, stdout, Harness};

#[test]
fn no_args_prints_top_help() {
    let h = Harness::new();
    let out = h.run(&[]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("Usage: rusta"));
}

#[test]
fn help_flag_prints_help() {
    let h = Harness::new();
    let out = h.run(&["--help"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("Usage: rusta"));
    assert!(stdout(&out).contains("Commands:"));
}

#[test]
fn subcommand_help_works() {
    let h = Harness::new();
    let out = h.run(&["create", "--help"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("--gui"));
}

#[test]
fn unknown_flag_errors() {
    let h = Harness::new();
    let out = h.run(&["--definitely-not-a-flag"]);
    assert_ne!(code(&out), 0);
    assert!(!stderr(&out).is_empty());
}

#[test]
fn completions_zsh_emits_compdef_header() {
    let h = Harness::new();
    let out = h.run(&["completions", "zsh"]);
    assert_eq!(code(&out), 0);
    assert!(
        stdout(&out).starts_with("#compdef rusta"),
        "expected zsh #compdef header, got: {:.120}",
        stdout(&out)
    );
}

#[test]
fn man_writes_root_and_subcommand_pages() {
    let h = Harness::new();
    let dir = h.root.join("man");
    let out = h.run(&["man", dir.to_str().unwrap()]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(dir.join("rusta.1").exists(), "root man page missing");
    assert!(dir.join("rusta-create.1").exists(), "rusta-create.1 missing");
    assert!(!dir.join("rusta-completions.1").exists(), "hidden cmd leaked");
}

#[test]
fn completions_and_man_are_hidden_from_top_help() {
    let h = Harness::new();
    let out = h.run(&["--help"]);
    assert_eq!(code(&out), 0);
    let s = stdout(&out);
    assert!(!s.contains("completions"), "completions should be hidden");
    assert!(!s.contains("\n  man "), "man should be hidden");
}
