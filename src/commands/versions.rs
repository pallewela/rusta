use std::io::IsTerminal;
use std::time::Duration;

use serde_json::Value;

use crate::error::{Error, Result};

const DEFAULT_VERSION: &str = "24.04";
const TOKEN_URL: &str = "https://ghcr.io/token?scope=repository:cirruslabs/ubuntu:pull";
const TAGS_URL: &str = "https://ghcr.io/v2/cirruslabs/ubuntu/tags/list";

fn token_url() -> String {
    std::env::var("RUSTA_GHCR_TOKEN_URL").unwrap_or_else(|_| TOKEN_URL.to_string())
}

fn tags_url() -> String {
    std::env::var("RUSTA_GHCR_TAGS_URL").unwrap_or_else(|_| TAGS_URL.to_string())
}

pub fn run() -> Result<u8> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .build();

    let token_resp: Value = agent
        .get(&token_url())
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
        .get(&tags_url())
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| Error::msg(format!("ghcr.io tags request failed: {e}")))?
        .into_json()
        .map_err(|e| Error::msg(format!("ghcr.io tags response parse: {e}")))?;
    let tags = tags_resp
        .get("tags")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::msg("ghcr.io tags response missing `tags` array".to_string()))?;

    let mut versions: Vec<(u32, u32, String)> = tags
        .iter()
        .filter_map(Value::as_str)
        .filter_map(parse_version)
        .collect();
    versions.sort();

    let tty = std::io::stdout().is_terminal();
    let (green, reset) = if tty {
        ("\x1b[0;32m", "\x1b[0m")
    } else {
        ("", "")
    };

    for (_, _, v) in &versions {
        if v == DEFAULT_VERSION {
            println!("{green}{v}{reset} (default)");
        } else {
            println!("{v}");
        }
    }
    Ok(0)
}

pub(crate) fn parse_version(t: &str) -> Option<(u32, u32, String)> {
    let (a, b) = t.split_once('.')?;
    let major: u32 = a.parse().ok()?;
    let minor: u32 = b.parse().ok()?;
    if a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|c| c.is_ascii_digit()) {
        Some((major, minor, t.to_string()))
    } else {
        None
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
}
