//! Internal library form of `rusta-cli`.
//!
//! Exists so the fuzz harness under `fuzz/` can call into the project's
//! parsers (e.g. [`tart::parse_list_json`]) without duplicating the
//! source. The module surface here is intentionally not a stable API:
//! external consumers should install the binary, not depend on this
//! library.

#![doc(hidden)]

pub mod cli;
pub mod commands;
pub mod error;
pub mod io;
pub mod paths;
pub mod picker;
pub mod provision;
pub mod runtime;
pub mod ssh;
pub mod state;
pub mod tart;
pub mod update_check;
