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
