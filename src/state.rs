use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_vm: Option<String>,
}

impl State {
    pub fn load() -> Self {
        let path = paths::state_file();
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        paths::ensure_dirs()?;
        let s = toml::to_string(self).expect("serialize state");
        write_atomically(&paths::state_file(), &s)
    }
}

fn write_atomically(path: &Path, contents: &str) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{}.tmp", path.file_name().unwrap().to_string_lossy()));
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

pub fn set_default(vm: &str) -> std::io::Result<()> {
    let mut s = State::load();
    s.default_vm = Some(vm.to_string());
    s.save()
}

pub fn clear_default_if_matches(vm: &str) -> std::io::Result<()> {
    let mut s = State::load();
    if s.default_vm.as_deref() == Some(vm) {
        s.default_vm = None;
        s.save()
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_root<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("RUSTA_STATE_ROOT");
        std::env::set_var("RUSTA_STATE_ROOT", tmp.path());
        f();
        match prev {
            Some(v) => std::env::set_var("RUSTA_STATE_ROOT", v),
            None => std::env::remove_var("RUSTA_STATE_ROOT"),
        }
    }

    #[test]
    fn load_returns_default_when_missing() {
        with_temp_root(|| {
            let s = State::load();
            assert!(s.default_vm.is_none());
        });
    }

    #[test]
    fn save_and_reload_roundtrip() {
        with_temp_root(|| {
            set_default("hello").unwrap();
            let s = State::load();
            assert_eq!(s.default_vm.as_deref(), Some("hello"));
        });
    }

    #[test]
    fn clear_default_only_when_match() {
        with_temp_root(|| {
            set_default("a").unwrap();
            clear_default_if_matches("b").unwrap();
            assert_eq!(State::load().default_vm.as_deref(), Some("a"));
            clear_default_if_matches("a").unwrap();
            assert!(State::load().default_vm.is_none());
        });
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        with_temp_root(|| {
            paths::ensure_dirs().unwrap();
            std::fs::write(paths::state_file(), b"@@@not toml@@@").unwrap();
            let s = State::load();
            assert!(s.default_vm.is_none());
        });
    }
}
