use std::collections::{BTreeMap, BTreeSet};
use std::io::IsTerminal;

use crate::cli::VersionsArgs;
use crate::error::{Error, Result};
use crate::io as rio;
use crate::registry;
use crate::state::{self, Source};

const DEFAULT_VERSION: &str = "24.04";

/// One source's contribution: its label and its ascending `(major, minor, tag)` versions.
type SourceVersions = (String, Vec<(u32, u32, String)>);

/// One image's contribution to the matrix: its name and the per-source versions
/// (in source-priority order) for that image.
type ImageCells = (String, Vec<SourceVersions>);

pub fn run(args: VersionsArgs) -> Result<u8> {
    let sources = select_sources(args.source.as_deref())?;
    let images = select_images(args.image.as_deref())?;

    // One image (the common case, including the default config) keeps the
    // original single-image rendering exactly. Multiple images enumerate the
    // source × image matrix.
    if images.len() <= 1 {
        let image = images
            .first()
            .map(String::as_str)
            .unwrap_or(state::DEFAULT_IMAGE);
        run_single_image(image, &sources)
    } else {
        run_matrix(&images, &sources)
    }
}

/// List versions for a single image across the given sources. Mirrors the
/// pre-image behavior: a single source is unannotated; multiple sources are
/// annotated with their provider(s) and the conflict winner.
fn run_single_image(image: &str, sources: &[Source]) -> Result<u8> {
    let multi = sources.len() > 1;

    let mut per_source: Vec<SourceVersions> = Vec::new();
    let mut errors: Vec<(String, String)> = Vec::new();
    for s in sources {
        match registry::image_versions(s, image) {
            Ok(vs) => per_source.push((s.label().to_string(), vs)),
            Err(e) => errors.push((s.label().to_string(), e.message)),
        }
    }

    if per_source.is_empty() {
        // Single source, or every source unreachable: surface the error(s).
        let detail = errors
            .iter()
            .map(|(l, e)| format!("{l}: {e}"))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Error::msg(format!(
            "could not list versions from any source ({detail})"
        )));
    }

    // At least one source responded; warn about any that didn't (only meaningful
    // when more than one was queried).
    if multi {
        for (label, e) in &errors {
            rio::skip(&format!("source '{label}' unreachable, skipping: {e}"));
        }
    }

    let (green, reset) = colors();
    for line in render_versions(&per_source, DEFAULT_VERSION, green, reset, multi) {
        println!("{line}");
    }
    Ok(0)
}

/// List versions across the full source × image matrix. A cell where a source
/// simply lacks an image is an empty cell (not an error, not a warning); a cell
/// where the source host is unreachable is warned about. Fails only when no cell
/// produced any result at all.
fn run_matrix(images: &[String], sources: &[Source]) -> Result<u8> {
    let mut per_image: Vec<ImageCells> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut any_ok = false;

    for image in images {
        let mut per_source: Vec<SourceVersions> = Vec::new();
        for s in sources {
            match registry::image_versions(s, image) {
                Ok(vs) => {
                    any_ok = true;
                    per_source.push((s.label().to_string(), vs));
                }
                Err(e) => {
                    if !is_absent_error(&e.message) {
                        warnings.push(format!(
                            "source '{}' image '{image}' unreachable, skipping: {}",
                            s.label(),
                            e.message
                        ));
                    }
                }
            }
        }
        per_image.push((image.clone(), per_source));
    }

    if !any_ok {
        return Err(Error::msg(
            "could not list versions for any configured image/source".to_string(),
        ));
    }

    for w in &warnings {
        rio::skip(w);
    }

    let (green, reset) = colors();
    for line in render_matrix(&per_image, DEFAULT_VERSION, green, reset) {
        println!("{line}");
    }
    Ok(0)
}

fn colors() -> (&'static str, &'static str) {
    if std::io::stdout().is_terminal() {
        ("\x1b[0;32m", "\x1b[0m")
    } else {
        ("", "")
    }
}

/// Heuristic: does a fetch error mean "this source simply doesn't host this
/// image" (a 404 / NAME_UNKNOWN / denied) rather than "the host is unreachable"?
/// Absent images are an expected, silent outcome in the matrix; everything else
/// is surfaced as a warning.
fn is_absent_error(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("404") || m.contains("name_unknown") || m.contains("not found") || m.contains("denied")
}

fn select_sources(filter: Option<&str>) -> Result<Vec<Source>> {
    let all = state::sources();
    match filter {
        None => Ok(all),
        Some(reg) => {
            let norm = registry::normalize_registry(reg);
            all.into_iter()
                .find(|s| s.registry == norm || s.label() == reg)
                .map(|s| vec![s])
                .ok_or_else(|| {
                    Error::msg(format!(
                        "source '{reg}' is not configured (see `rusta source list`)"
                    ))
                })
        }
    }
}

/// Images to enumerate: all configured (priority order) or, when `--image` is
/// given, just that one (validated; need not be in the configured list).
fn select_images(filter: Option<&str>) -> Result<Vec<String>> {
    match filter {
        None => Ok(state::images()),
        Some(name) => Ok(vec![registry::validate_image(name)?]),
    }
}

/// Build printable version lines. `per_source` is `(label, ascending versions)`
/// in priority order. With a single source the output is unannotated (legacy
/// format); with multiple sources each version is annotated with its provider(s)
/// and the source `create` would pick on conflict (first in priority order).
pub fn render_versions(
    per_source: &[SourceVersions],
    default_version: &str,
    green: &str,
    reset: &str,
    multi: bool,
) -> Vec<String> {
    let mut providers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut order: Vec<(u32, u32, String)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (label, versions) in per_source {
        for (maj, min, tag) in versions {
            providers
                .entry(tag.clone())
                .or_default()
                .push(label.clone());
            if seen.insert(tag.clone()) {
                order.push((*maj, *min, tag.clone()));
            }
        }
    }
    order.sort();

    let mut lines = Vec::new();
    for (_, _, v) in &order {
        let is_default = v == default_version;
        let vstr = if is_default {
            format!("{green}{v}{reset}")
        } else {
            v.clone()
        };
        let default_suffix = if is_default { " (default)" } else { "" };
        if !multi {
            lines.push(format!("{vstr}{default_suffix}"));
        } else {
            let provs = providers.get(v).cloned().unwrap_or_default();
            let chosen = if provs.len() > 1 {
                format!("  (create uses {})", provs[0])
            } else {
                String::new()
            };
            lines.push(format!(
                "{vstr}{default_suffix}   from: {}{chosen}",
                provs.join(", ")
            ));
        }
    }
    lines
}

/// Build printable lines for the source × image matrix. Each version line lists,
/// per image (in priority order) that provides it, the source(s) advertising it
/// (in priority order); an image with more than one provider is annotated with
/// the source `create` would pick (first by priority).
pub fn render_matrix(
    per_image: &[ImageCells],
    default_version: &str,
    green: &str,
    reset: &str,
) -> Vec<String> {
    // Collect the union of versions across the whole matrix, ascending.
    let mut order: Vec<(u32, u32, String)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (_image, per_source) in per_image {
        for (_label, versions) in per_source {
            for (maj, min, tag) in versions {
                if seen.insert(tag.clone()) {
                    order.push((*maj, *min, tag.clone()));
                }
            }
        }
    }
    order.sort();

    let mut lines = Vec::new();
    for (_, _, v) in &order {
        let is_default = v == default_version;
        let vstr = if is_default {
            format!("{green}{v}{reset}")
        } else {
            v.clone()
        };
        let default_suffix = if is_default { " (default)" } else { "" };

        let mut segments = Vec::new();
        for (image, per_source) in per_image {
            let labels: Vec<String> = per_source
                .iter()
                .filter(|(_l, versions)| versions.iter().any(|(_, _, t)| t == v))
                .map(|(l, _)| l.clone())
                .collect();
            if labels.is_empty() {
                continue;
            }
            let mut seg = format!("{image}: {}", labels.join(", "));
            if labels.len() > 1 {
                seg.push_str(&format!("  (create uses {})", labels[0]));
            }
            segments.push(seg);
        }
        lines.push(format!("{vstr}{default_suffix}   {}", segments.join("   ")));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(tag: &str) -> (u32, u32, String) {
        registry::parse_version(tag).unwrap()
    }

    #[test]
    fn single_source_uses_legacy_format() {
        let per = vec![(
            "cirruslabs".to_string(),
            vec![v("20.04"), v("22.04"), v("24.04")],
        )];
        let lines = render_versions(&per, "24.04", "", "", false);
        assert_eq!(lines, vec!["20.04", "22.04", "24.04 (default)"]);
    }

    #[test]
    fn multi_source_annotates_and_marks_chosen() {
        let per = vec![
            ("cirruslabs".to_string(), vec![v("22.04"), v("24.04")]),
            ("pallewela".to_string(), vec![v("22.04"), v("25.04")]),
        ];
        let lines = render_versions(&per, "24.04", "", "", true);
        assert!(
            lines
                .iter()
                .any(|l| l == "22.04   from: cirruslabs, pallewela  (create uses cirruslabs)"),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l == "24.04 (default)   from: cirruslabs"),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l == "25.04   from: pallewela"),
            "{lines:?}"
        );
    }

    #[test]
    fn versions_sorted_ascending_across_sources() {
        let per = vec![
            ("a".to_string(), vec![v("24.04")]),
            ("b".to_string(), vec![v("20.04")]),
        ];
        let lines = render_versions(&per, "24.04", "", "", true);
        assert!(lines[0].starts_with("20.04"), "{lines:?}");
        assert!(lines[1].starts_with("24.04"), "{lines:?}");
    }

    #[test]
    fn matrix_groups_by_image_and_marks_conflicts() {
        let per_image = vec![
            (
                "ubuntu".to_string(),
                vec![
                    ("cirruslabs".to_string(), vec![v("22.04"), v("24.04")]),
                    ("pallewela".to_string(), vec![v("22.04")]),
                ],
            ),
            (
                "ubuntu-desktop".to_string(),
                vec![("pallewela".to_string(), vec![v("24.04"), v("25.04")])],
            ),
        ];
        let lines = render_matrix(&per_image, "24.04", "", "");
        assert_eq!(
            lines,
            vec![
                "22.04   ubuntu: cirruslabs, pallewela  (create uses cirruslabs)".to_string(),
                "24.04 (default)   ubuntu: cirruslabs   ubuntu-desktop: pallewela".to_string(),
                "25.04   ubuntu-desktop: pallewela".to_string(),
            ]
        );
    }

    #[test]
    fn matrix_omits_images_without_a_version() {
        let per_image = vec![
            ("ubuntu".to_string(), vec![("cirruslabs".to_string(), vec![v("24.04")])]),
            ("ubuntu-desktop".to_string(), vec![("pallewela".to_string(), vec![])]),
        ];
        let lines = render_matrix(&per_image, "24.04", "", "");
        assert_eq!(
            lines,
            vec!["24.04 (default)   ubuntu: cirruslabs".to_string()]
        );
    }

    #[test]
    fn absent_error_distinguished_from_unreachable() {
        assert!(is_absent_error("ghcr.io tags request failed: status code 404"));
        assert!(is_absent_error("denied: requested access to the resource is denied"));
        assert!(is_absent_error("NAME_UNKNOWN"));
        assert!(!is_absent_error("ghcr.io token request failed: connection refused"));
        assert!(!is_absent_error("dns error: failed to lookup address"));
    }
}
