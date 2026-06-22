//! ghcr.io OCI registry client for Ubuntu image sources.
//!
//! Builds the token + tag-list URLs for a given [`Source`] and image name,
//! fetches the available `<namespace>/<image>` tags, and provides the pure
//! resolution logic used by `create` to pick which source advertises a
//! requested version.
//!
//! v1 supports ghcr.io only; other registries would need their own auth flow.

use std::time::Duration;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::state::Source;

/// Token URL. Honors `RUSTA_GHCR_TOKEN_URL` (test hook; applies to every source).
fn token_url(host: &str, repo: &str) -> String {
    std::env::var("RUSTA_GHCR_TOKEN_URL")
        .unwrap_or_else(|_| format!("https://{host}/token?scope=repository:{repo}:pull"))
}

/// Tags-list URL. Honors `RUSTA_GHCR_TAGS_URL` (test hook; applies to every source).
fn tags_url(host: &str, repo: &str) -> String {
    std::env::var("RUSTA_GHCR_TAGS_URL")
        .unwrap_or_else(|_| format!("https://{host}/v2/{repo}/tags/list"))
}

/// Fetch the raw tag list for `<source>/<image>` from the registry.
pub fn fetch_tags(source: &Source, image: &str) -> Result<Vec<String>> {
    let (host, repo) = source.host_and_repo_path(image).ok_or_else(|| {
        Error::msg(format!(
            "invalid source '{}': expected <host>/<namespace>",
            source.registry
        ))
    })?;

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .build();

    let token_resp: Value = agent
        .get(&token_url(host, &repo))
        .call()
        .map_err(|e| Error::msg(format!("ghcr.io token request failed: {e}")))?
        .into_json()
        .map_err(|e| Error::msg(format!("ghcr.io token response parse: {e}")))?;
    let token = token_resp
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::msg("ghcr.io did not return a pull token".to_string()))?
        .to_string();

    let tags_resp: Value = agent
        .get(&tags_url(host, &repo))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| Error::msg(format!("ghcr.io tags request failed: {e}")))?
        .into_json()
        .map_err(|e| Error::msg(format!("ghcr.io tags response parse: {e}")))?;
    let tags = tags_resp
        .get("tags")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::msg("ghcr.io tags response missing `tags` array".to_string()))?;

    Ok(tags
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect())
}

/// Parsed, ascending-sorted `X.Y` versions advertised by `<source>/<image>`.
pub fn image_versions(source: &Source, image: &str) -> Result<Vec<(u32, u32, String)>> {
    let mut versions: Vec<(u32, u32, String)> = fetch_tags(source, image)?
        .iter()
        .filter_map(|t| parse_version(t))
        .collect();
    versions.sort();
    Ok(versions)
}

/// Parse an `X.Y` Ubuntu tag (both numeric) into `(major, minor, tag)`.
pub fn parse_version(t: &str) -> Option<(u32, u32, String)> {
    let (a, b) = t.split_once('.')?;
    let major: u32 = a.parse().ok()?;
    let minor: u32 = b.parse().ok()?;
    if a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|c| c.is_ascii_digit()) {
        Some((major, minor, t.to_string()))
    } else {
        None
    }
}

/// Strip surrounding whitespace, a trailing `/ubuntu`, and trailing slashes from
/// a user-supplied source string, yielding the canonical registry prefix.
pub fn normalize_registry(input: &str) -> String {
    let mut s = input.trim().trim_end_matches('/');
    if let Some(stripped) = s.strip_suffix("/ubuntu") {
        s = stripped.trim_end_matches('/');
    }
    s.to_string()
}

/// Validate + normalize a source prefix for v1 (ghcr.io, `<host>/<namespace>`,
/// no tag). Returns the canonical prefix to store.
pub fn validate_registry(input: &str) -> Result<String> {
    let s = normalize_registry(input);
    if s.contains(':') {
        return Err(Error::msg(format!(
            "source must not include a tag (':'); the tag comes from --version (got '{input}')"
        )));
    }
    let Some((host, ns)) = s.split_once('/') else {
        return Err(Error::msg(format!(
            "source must be of the form <host>/<namespace>, e.g. ghcr.io/pallewela (got '{input}')"
        )));
    };
    if host.is_empty() || ns.is_empty() {
        return Err(Error::msg(format!(
            "source must include a namespace, e.g. ghcr.io/pallewela (got '{input}')"
        )));
    }
    if host != "ghcr.io" {
        return Err(Error::msg(format!(
            "only ghcr.io sources are supported in this version (got host '{host}')"
        )));
    }
    Ok(s)
}

/// Validate + normalize an image name (the repository segment under a source,
/// e.g. `ubuntu`, `ubuntu-desktop`). An image is a single OCI path segment: no
/// slash (the namespace comes from the source), no tag, lowercase grammar.
/// Returns the canonical name to store.
pub fn validate_image(input: &str) -> Result<String> {
    let s = input.trim();
    if s.is_empty() {
        return Err(Error::msg("image name must not be empty".to_string()));
    }
    if s.contains(':') {
        return Err(Error::msg(format!(
            "image must not include a tag (':'); the tag comes from --version (got '{input}')"
        )));
    }
    if s.contains('/') {
        return Err(Error::msg(format!(
            "image must be a single repository name, not a path; the namespace comes from the source (got '{input}')"
        )));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
    {
        return Err(Error::msg(format!(
            "invalid image name '{input}' (use lowercase letters, digits, '-', '_', '.')"
        )));
    }
    Ok(s.to_string())
}

/// Outcome of resolving a version across candidate sources.
pub struct Pick {
    /// The selected image reference, if a reachable source advertised the version.
    pub image: Option<String>,
    /// Labels of unreachable sources encountered before the decision — the
    /// caller emits a warning for each.
    pub warn: Vec<String>,
    /// Error message when no reachable source advertised the version.
    pub err: Option<String>,
}

/// Pick the first source (in priority order) whose tag list contains `version`
/// for the given `image`. `results` pairs each source with its fetch outcome
/// (`Ok(tags)` or `Err(reason)`). Stops at the first match, so sources after it
/// are neither warned about nor relied on.
pub fn pick_image(
    image: &str,
    version: &str,
    results: &[(Source, std::result::Result<Vec<String>, String>)],
) -> Pick {
    let mut warn = Vec::new();
    let mut checked = Vec::new();
    for (source, result) in results {
        match result {
            Ok(tags) => {
                checked.push(source.label().to_string());
                if tags.iter().any(|t| t == version) {
                    return Pick {
                        image: Some(source.image_ref(image, version)),
                        warn,
                        err: None,
                    };
                }
            }
            Err(_) => warn.push(source.label().to_string()),
        }
    }
    let mut msg = format!("{image} {version} not found in any reachable configured source");
    if !checked.is_empty() {
        msg.push_str(&format!(" (checked: {})", checked.join(", ")));
    }
    if !warn.is_empty() {
        msg.push_str(&format!("; unreachable: {}", warn.join(", ")));
    }
    Pick {
        image: None,
        warn,
        err: Some(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_accepts_two_dot() {
        assert_eq!(parse_version("24.04"), Some((24, 4, "24.04".into())));
        assert_eq!(parse_version("22.10"), Some((22, 10, "22.10".into())));
    }

    #[test]
    fn parse_version_rejects_non_numeric() {
        assert!(parse_version("latest").is_none());
        assert!(parse_version("24").is_none());
        assert!(parse_version("24.04.1").is_none());
        assert!(parse_version("24.x").is_none());
    }

    #[test]
    fn normalize_strips_ubuntu_and_slashes() {
        assert_eq!(
            normalize_registry("  ghcr.io/pallewela  "),
            "ghcr.io/pallewela"
        );
        assert_eq!(
            normalize_registry("ghcr.io/pallewela/"),
            "ghcr.io/pallewela"
        );
        assert_eq!(
            normalize_registry("ghcr.io/pallewela/ubuntu"),
            "ghcr.io/pallewela"
        );
        assert_eq!(
            normalize_registry("ghcr.io/pallewela/ubuntu/"),
            "ghcr.io/pallewela"
        );
    }

    #[test]
    fn validate_accepts_ghcr_prefix() {
        assert_eq!(
            validate_registry("ghcr.io/pallewela/ubuntu").unwrap(),
            "ghcr.io/pallewela"
        );
        assert_eq!(
            validate_registry("ghcr.io/cirruslabs").unwrap(),
            "ghcr.io/cirruslabs"
        );
    }

    #[test]
    fn validate_rejects_bad_inputs() {
        assert!(validate_registry("pallewela").is_err()); // no host/namespace
        assert!(validate_registry("ghcr.io/").is_err()); // empty namespace
        assert!(validate_registry("ghcr.io/pallewela:22.04").is_err()); // tag present
        assert!(validate_registry("docker.io/library").is_err()); // non-ghcr host
    }

    #[test]
    fn validate_image_accepts_bare_names() {
        assert_eq!(validate_image("ubuntu").unwrap(), "ubuntu");
        assert_eq!(validate_image("  ubuntu-desktop  ").unwrap(), "ubuntu-desktop");
        assert_eq!(validate_image("ubuntu_22.04-base").unwrap(), "ubuntu_22.04-base");
    }

    #[test]
    fn validate_image_rejects_bad_inputs() {
        assert!(validate_image("").is_err()); // empty
        assert!(validate_image("   ").is_err()); // whitespace only
        assert!(validate_image("ubuntu:22.04").is_err()); // tag present
        assert!(validate_image("pallewela/ubuntu").is_err()); // path, not a bare name
        assert!(validate_image("Ubuntu").is_err()); // uppercase
        assert!(validate_image("ubuntu desktop").is_err()); // space
    }

    fn src(reg: &str) -> Source {
        Source::new(reg)
    }

    #[test]
    fn pick_first_source_with_version_wins() {
        let results = vec![
            (
                src("ghcr.io/cirruslabs"),
                Ok(vec!["22.04".into(), "24.04".into()]),
            ),
            (src("ghcr.io/pallewela"), Ok(vec!["22.04".into()])),
        ];
        let pick = pick_image("ubuntu", "22.04", &results);
        assert_eq!(
            pick.image.as_deref(),
            Some("ghcr.io/cirruslabs/ubuntu:22.04")
        );
        assert!(pick.warn.is_empty());
        assert!(pick.err.is_none());
    }

    #[test]
    fn pick_skips_unreachable_then_matches() {
        let results = vec![
            (src("ghcr.io/cirruslabs"), Err("boom".to_string())),
            (src("ghcr.io/pallewela"), Ok(vec!["22.04".into()])),
        ];
        let pick = pick_image("ubuntu", "22.04", &results);
        assert_eq!(
            pick.image.as_deref(),
            Some("ghcr.io/pallewela/ubuntu:22.04")
        );
        assert_eq!(pick.warn, vec!["cirruslabs"]);
    }

    #[test]
    fn pick_not_found_reports_checked_and_unreachable() {
        let results = vec![
            (src("ghcr.io/cirruslabs"), Ok(vec!["24.04".into()])),
            (src("ghcr.io/pallewela"), Err("down".to_string())),
        ];
        let pick = pick_image("ubuntu", "22.04", &results);
        assert!(pick.image.is_none());
        let err = pick.err.unwrap();
        assert!(err.contains("checked: cirruslabs"), "{err}");
        assert!(err.contains("unreachable: pallewela"), "{err}");
    }

    #[test]
    fn pick_does_not_warn_about_sources_after_match() {
        let results = vec![
            (src("ghcr.io/cirruslabs"), Ok(vec!["22.04".into()])),
            (src("ghcr.io/pallewela"), Err("down".to_string())),
        ];
        let pick = pick_image("ubuntu", "22.04", &results);
        assert_eq!(
            pick.image.as_deref(),
            Some("ghcr.io/cirruslabs/ubuntu:22.04")
        );
        assert!(
            pick.warn.is_empty(),
            "should not warn about pallewela after match"
        );
    }
}
