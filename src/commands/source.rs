use crate::cli::{SourceAction, SourceArgs};
use crate::error::{Error, Result};
use crate::io as rio;
use crate::registry;
use crate::state;

pub fn run(args: SourceArgs) -> Result<u8> {
    match args.action.unwrap_or(SourceAction::List) {
        SourceAction::List => list(),
        SourceAction::Add { registry } => add(&registry),
        SourceAction::Rm { registry } => rm(&registry),
        SourceAction::Move { registry, position } => reorder(&registry, position),
    }
}

fn list() -> Result<u8> {
    let configured = !state::State::load().sources.is_empty();
    let sources = state::sources();
    rio::info("Image sources (priority order):");
    for (i, s) in sources.iter().enumerate() {
        println!("  {}. {}  ({})", i + 1, s.registry, s.label());
    }
    if !configured {
        println!();
        rio::skip(&format!(
            "Using the built-in default ({}). Add one with `rusta source add <registry>`.",
            state::DEFAULT_SOURCE_REGISTRY
        ));
    }
    Ok(0)
}

fn add(input: &str) -> Result<u8> {
    let registry = registry::validate_registry(input)?;
    if state::add_source(&registry).map_err(|e| Error::msg(e.to_string()))? {
        rio::ok(&format!("Added source: {registry}"));
    } else {
        rio::skip(&format!("Source already configured: {registry}"));
    }
    Ok(0)
}

fn rm(input: &str) -> Result<u8> {
    let registry = registry::normalize_registry(input);
    if state::remove_source(&registry).map_err(|e| Error::msg(e.to_string()))? {
        rio::ok(&format!("Removed source: {registry}"));
        Ok(0)
    } else {
        Err(Error::not_found(format!(
            "source not configured: {registry}"
        )))
    }
}

fn reorder(input: &str, position: usize) -> Result<u8> {
    let registry = registry::normalize_registry(input);
    if state::move_source(&registry, position).map_err(|e| Error::msg(e.to_string()))? {
        rio::ok(&format!("Moved source {registry} to position {position}"));
        Ok(0)
    } else {
        Err(Error::not_found(format!(
            "source not configured: {registry}"
        )))
    }
}
