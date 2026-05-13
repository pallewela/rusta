//! Runtime knobs that let tests redirect external binaries to fakes.
//!
//! For each external binary `rusta` shells out to, a corresponding env var
//! `RUSTA_<UPPER>_BIN` can override the path. This is the seam that
//! integration tests use to substitute deterministic fake binaries
//! (typically shell scripts) for `tart`, `sshpass`, `ssh`, etc.

/// Resolve a binary name to a path, honoring `RUSTA_<NAME>_BIN` overrides.
pub fn bin_for(name: &str) -> String {
    let key = format!(
        "RUSTA_{}_BIN",
        name.to_ascii_uppercase().replace('-', "_")
    );
    std::env::var(key).unwrap_or_else(|_| name.to_string())
}

/// Tests can set `RUSTA_SKIP_PREFLIGHT=1` to bypass the arm64/brew/tart checks.
pub fn skip_preflight() -> bool {
    std::env::var("RUSTA_SKIP_PREFLIGHT").is_ok()
}

/// Optional override for the state/provision/run directory root.
/// When set, all rusta state paths resolve under this directory instead of $HOME.
pub fn state_root_override() -> Option<std::path::PathBuf> {
    std::env::var("RUSTA_STATE_ROOT")
        .ok()
        .map(std::path::PathBuf::from)
}

/// Optional override for the SSH directory root (defaults to $HOME/.ssh).
pub fn ssh_dir_override() -> Option<std::path::PathBuf> {
    std::env::var("RUSTA_SSH_DIR")
        .ok()
        .map(std::path::PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn bin_for_falls_back_when_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("RUSTA_TART_BIN");
        assert_eq!(bin_for("tart"), "tart");
    }

    #[test]
    fn bin_for_honors_override() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("RUSTA_TART_BIN", "/path/to/fake-tart");
        assert_eq!(bin_for("tart"), "/path/to/fake-tart");
        std::env::remove_var("RUSTA_TART_BIN");
    }

    #[test]
    fn bin_for_replaces_hyphen_in_env_key() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("RUSTA_SSH_COPY_ID_BIN", "/fake/ssh-copy-id");
        assert_eq!(bin_for("ssh-copy-id"), "/fake/ssh-copy-id");
        std::env::remove_var("RUSTA_SSH_COPY_ID_BIN");
    }
}
