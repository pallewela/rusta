#![no_main]
//! Fuzz harness for `tart::parse_list_json`.
//!
//! The function parses the JSON output of `tart list --format json`,
//! which in deployment comes from an external process. A malformed or
//! adversarial payload must not panic or crash the CLI; a `Result::Err`
//! is the expected failure mode.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = rusta_cli::tart::parse_list_json(data);
});
