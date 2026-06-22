use crate::cli::{ImageAction, ImageArgs};
use crate::error::{Error, Result};
use crate::io as rio;
use crate::registry;
use crate::state;

pub fn run(args: ImageArgs) -> Result<u8> {
    match args.action.unwrap_or(ImageAction::List) {
        ImageAction::List => list(),
        ImageAction::Add { name } => add(&name),
        ImageAction::Rm { name } => rm(&name),
        ImageAction::Move { name, position } => reorder(&name, position),
    }
}

fn list() -> Result<u8> {
    let configured = !state::State::load().images.is_empty();
    let images = state::images();
    rio::info("Images (priority order; first is the create default):");
    for (i, name) in images.iter().enumerate() {
        let marker = if i == 0 { "  (default)" } else { "" };
        println!("  {}. {}{}", i + 1, name, marker);
    }
    if !configured {
        println!();
        rio::skip(&format!(
            "Using the built-in defaults ({}). Add one with `rusta image add <name>`.",
            state::DEFAULT_IMAGES.join(", ")
        ));
    }
    Ok(0)
}

fn add(input: &str) -> Result<u8> {
    let name = registry::validate_image(input)?;
    if state::add_image(&name).map_err(|e| Error::msg(e.to_string()))? {
        rio::ok(&format!("Added image: {name}"));
    } else {
        rio::skip(&format!("Image already configured: {name}"));
    }
    Ok(0)
}

fn rm(input: &str) -> Result<u8> {
    let name = input.trim();
    if state::remove_image(name).map_err(|e| Error::msg(e.to_string()))? {
        rio::ok(&format!("Removed image: {name}"));
        Ok(0)
    } else {
        Err(Error::not_found(format!("image not configured: {name}")))
    }
}

fn reorder(input: &str, position: usize) -> Result<u8> {
    let name = input.trim();
    if state::move_image(name, position).map_err(|e| Error::msg(e.to_string()))? {
        rio::ok(&format!("Moved image {name} to position {position}"));
        Ok(0)
    } else {
        Err(Error::not_found(format!("image not configured: {name}")))
    }
}
