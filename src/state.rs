use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::paths;

/// The image source seeded when no sources are configured. Preserves the
/// original single-source behavior (clone Ubuntu from cirruslabs).
pub const DEFAULT_SOURCE_REGISTRY: &str = "ghcr.io/cirruslabs";

/// The primary image name — the `create` default and the first seeded image.
pub const DEFAULT_IMAGE: &str = "ubuntu";

/// The image names seeded when none are configured, in priority order. The
/// first (`ubuntu`) is the `create` default; `ubuntu-desktop` is supported
/// out of the box too.
pub const DEFAULT_IMAGES: &[&str] = &["ubuntu", "ubuntu-desktop"];

/// The seeded image list as owned strings.
fn default_images() -> Vec<String> {
    DEFAULT_IMAGES.iter().map(|s| s.to_string()).collect()
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_vm: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vms: BTreeMap<String, VmState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<UpdateState>,
    /// Ordered list of image sources (registry+namespace prefixes). Position is
    /// priority; the first source advertising a requested version wins. An empty
    /// list means "use the seeded default" — see [`State::effective_sources`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
    /// Ordered list of image names (repositories under each source). Position is
    /// priority; the first is the `create` default. An empty list means "use the
    /// seeded default" (`ubuntu`) — see [`State::effective_images`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
}

/// An image source: a registry host + namespace prefix, e.g. `ghcr.io/cirruslabs`.
/// rusta always appends `/ubuntu` to form the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub registry: String,
}

impl Source {
    pub fn new(registry: impl Into<String>) -> Self {
        Self {
            registry: registry.into(),
        }
    }

    /// Human-readable label: the last path segment of the prefix
    /// (`ghcr.io/cirruslabs` → `cirruslabs`). Not a unique identifier.
    pub fn label(&self) -> &str {
        self.registry.rsplit('/').next().unwrap_or(&self.registry)
    }

    /// Full image reference for a given image + version, e.g.
    /// `ghcr.io/cirruslabs/ubuntu:24.04`.
    pub fn image_ref(&self, image: &str, version: &str) -> String {
        format!("{}/{}:{}", self.registry, image, version)
    }

    /// `(host, "<namespace>/<image>")` for building registry v2 API URLs, e.g.
    /// `("ghcr.io", "cirruslabs/ubuntu")`. Returns `None` if there is no `/`.
    pub fn host_and_repo_path(&self, image: &str) -> Option<(&str, String)> {
        let (host, ns) = self.registry.split_once('/')?;
        Some((host, format!("{ns}/{image}")))
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VmState {
    #[serde(default)]
    pub gui: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    #[serde(default)]
    pub last_checked_at: u64,
    #[serde(default)]
    pub last_notified_at: u64,
    #[serde(default)]
    pub latest_known: Option<String>,
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

    /// Configured sources, or the seeded default when none are configured.
    /// rusta is never sourceless.
    pub fn effective_sources(&self) -> Vec<Source> {
        if self.sources.is_empty() {
            vec![Source::new(DEFAULT_SOURCE_REGISTRY)]
        } else {
            self.sources.clone()
        }
    }

    /// Configured images, or the seeded defaults (`ubuntu`, `ubuntu-desktop`)
    /// when none are configured. rusta is never imageless.
    pub fn effective_images(&self) -> Vec<String> {
        if self.images.is_empty() {
            default_images()
        } else {
            self.images.clone()
        }
    }
}

/// Effective image sources in priority order (seeded default when none configured).
pub fn sources() -> Vec<Source> {
    State::load().effective_sources()
}

/// Effective image names in priority order (seeded default when none configured).
pub fn images() -> Vec<String> {
    State::load().effective_images()
}

/// Materialize the seeded default into the stored list before a mutation, so the
/// user starts from `[cirruslabs]` and adds to it.
fn materialize(s: &mut State) {
    if s.sources.is_empty() {
        s.sources = s.effective_sources();
    }
}

/// Append a source. Returns `Ok(false)` if the registry is already present.
pub fn add_source(registry: &str) -> std::io::Result<bool> {
    let mut s = State::load();
    materialize(&mut s);
    if s.sources.iter().any(|x| x.registry == registry) {
        return Ok(false);
    }
    s.sources.push(Source::new(registry));
    s.save()?;
    Ok(true)
}

/// Remove a source by exact registry prefix. Returns `Ok(false)` if absent.
/// Removing the last remaining source re-seeds the default (never sourceless).
pub fn remove_source(registry: &str) -> std::io::Result<bool> {
    let mut s = State::load();
    materialize(&mut s);
    let before = s.sources.len();
    s.sources.retain(|x| x.registry != registry);
    if s.sources.len() == before {
        return Ok(false);
    }
    if s.sources.is_empty() {
        s.sources = vec![Source::new(DEFAULT_SOURCE_REGISTRY)];
    }
    s.save()?;
    Ok(true)
}

/// Move a source to a new 1-based priority position (clamped). Returns
/// `Ok(false)` if the registry is not present.
pub fn move_source(registry: &str, position: usize) -> std::io::Result<bool> {
    let mut s = State::load();
    materialize(&mut s);
    let Some(cur) = s.sources.iter().position(|x| x.registry == registry) else {
        return Ok(false);
    };
    let item = s.sources.remove(cur);
    let target = position.saturating_sub(1).min(s.sources.len());
    s.sources.insert(target, item);
    s.save()?;
    Ok(true)
}

/// Materialize the seeded default image into the stored list before a mutation,
/// so the user starts from `[ubuntu]` and adds to it.
fn materialize_images(s: &mut State) {
    if s.images.is_empty() {
        s.images = s.effective_images();
    }
}

/// Append an image. Returns `Ok(false)` if the name is already present.
pub fn add_image(name: &str) -> std::io::Result<bool> {
    let mut s = State::load();
    materialize_images(&mut s);
    if s.images.iter().any(|x| x == name) {
        return Ok(false);
    }
    s.images.push(name.to_string());
    s.save()?;
    Ok(true)
}

/// Remove an image by exact name. Returns `Ok(false)` if absent. Removing the
/// last remaining image re-seeds the default (never imageless).
pub fn remove_image(name: &str) -> std::io::Result<bool> {
    let mut s = State::load();
    materialize_images(&mut s);
    let before = s.images.len();
    s.images.retain(|x| x != name);
    if s.images.len() == before {
        return Ok(false);
    }
    if s.images.is_empty() {
        s.images = default_images();
    }
    s.save()?;
    Ok(true)
}

/// Move an image to a new 1-based priority position (clamped). Returns
/// `Ok(false)` if the name is not present.
pub fn move_image(name: &str, position: usize) -> std::io::Result<bool> {
    let mut s = State::load();
    materialize_images(&mut s);
    let Some(cur) = s.images.iter().position(|x| x == name) else {
        return Ok(false);
    };
    let item = s.images.remove(cur);
    let target = position.saturating_sub(1).min(s.images.len());
    s.images.insert(target, item);
    s.save()?;
    Ok(true)
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

pub fn set_vm_gui(vm: &str, gui: bool) -> std::io::Result<()> {
    let mut s = State::load();
    s.vms.insert(vm.to_string(), VmState { gui });
    s.save()
}

pub fn vm_gui(vm: &str) -> Option<bool> {
    State::load().vms.get(vm).map(|v| v.gui)
}

pub fn forget_vm(vm: &str) -> std::io::Result<()> {
    let mut s = State::load();
    if s.vms.remove(vm).is_some() {
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
    fn vm_gui_roundtrip() {
        with_temp_root(|| {
            assert_eq!(vm_gui("lab"), None);
            set_vm_gui("lab", true).unwrap();
            assert_eq!(vm_gui("lab"), Some(true));
            set_vm_gui("lab", false).unwrap();
            assert_eq!(vm_gui("lab"), Some(false));
            forget_vm("lab").unwrap();
            assert_eq!(vm_gui("lab"), None);
        });
    }

    #[test]
    fn old_schema_without_vms_table_still_loads() {
        with_temp_root(|| {
            paths::ensure_dirs().unwrap();
            std::fs::write(paths::state_file(), b"default_vm = \"hello\"\n").unwrap();
            let s = State::load();
            assert_eq!(s.default_vm.as_deref(), Some("hello"));
            assert!(s.vms.is_empty());
            assert_eq!(vm_gui("hello"), None);
        });
    }

    #[test]
    fn forget_vm_is_noop_when_absent() {
        with_temp_root(|| {
            forget_vm("missing").unwrap();
            assert!(State::load().vms.is_empty());
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

    #[test]
    fn source_label_and_refs() {
        let s = Source::new("ghcr.io/cirruslabs");
        assert_eq!(s.label(), "cirruslabs");
        assert_eq!(
            s.image_ref("ubuntu", "24.04"),
            "ghcr.io/cirruslabs/ubuntu:24.04"
        );
        assert_eq!(
            s.image_ref("ubuntu-desktop", "24.04"),
            "ghcr.io/cirruslabs/ubuntu-desktop:24.04"
        );
        assert_eq!(
            s.host_and_repo_path("ubuntu"),
            Some(("ghcr.io", "cirruslabs/ubuntu".to_string()))
        );
        assert_eq!(
            s.host_and_repo_path("ubuntu-desktop"),
            Some(("ghcr.io", "cirruslabs/ubuntu-desktop".to_string()))
        );
    }

    #[test]
    fn effective_images_seeds_defaults_when_empty() {
        with_temp_root(|| {
            assert_eq!(
                images(),
                vec!["ubuntu".to_string(), "ubuntu-desktop".to_string()]
            );
        });
    }

    #[test]
    fn add_image_materializes_defaults_then_appends() {
        with_temp_root(|| {
            assert!(add_image("ubuntu-kairos").unwrap());
            assert_eq!(
                images(),
                vec![
                    "ubuntu".to_string(),
                    "ubuntu-desktop".to_string(),
                    "ubuntu-kairos".to_string()
                ]
            );
        });
    }

    #[test]
    fn add_image_is_idempotent() {
        with_temp_root(|| {
            // ubuntu-desktop is already a seeded default → adding is a no-op.
            assert!(!add_image("ubuntu-desktop").unwrap());
            assert!(add_image("ubuntu-kairos").unwrap());
            assert!(!add_image("ubuntu-kairos").unwrap());
            assert_eq!(
                images().iter().filter(|i| *i == "ubuntu-kairos").count(),
                1
            );
        });
    }

    #[test]
    fn remove_image_and_reseed_when_last() {
        with_temp_root(|| {
            // Remove both seeded defaults; the list must never become empty.
            assert!(remove_image("ubuntu-desktop").unwrap());
            assert_eq!(images(), vec!["ubuntu".to_string()]);
            assert!(remove_image("ubuntu").unwrap());
            // Removing the last remaining image re-seeds the defaults.
            assert_eq!(
                images(),
                vec!["ubuntu".to_string(), "ubuntu-desktop".to_string()]
            );
            // Removing something absent is a no-op false.
            assert!(!remove_image("nope").unwrap());
        });
    }

    #[test]
    fn move_image_changes_priority() {
        with_temp_root(|| {
            add_image("ubuntu-kairos").unwrap();
            // [ubuntu, ubuntu-desktop, ubuntu-kairos] -> move kairos to position 1
            assert!(move_image("ubuntu-kairos", 1).unwrap());
            assert_eq!(
                images(),
                vec![
                    "ubuntu-kairos".to_string(),
                    "ubuntu".to_string(),
                    "ubuntu-desktop".to_string()
                ]
            );
            // Out-of-range position clamps to the end.
            assert!(move_image("ubuntu-kairos", 99).unwrap());
            assert_eq!(images().last().unwrap(), "ubuntu-kairos");
            assert!(!move_image("absent", 1).unwrap());
        });
    }

    #[test]
    fn old_schema_without_images_still_loads() {
        with_temp_root(|| {
            paths::ensure_dirs().unwrap();
            std::fs::write(paths::state_file(), b"default_vm = \"hello\"\n").unwrap();
            let s = State::load();
            assert!(s.images.is_empty());
            assert_eq!(
                images(),
                vec!["ubuntu".to_string(), "ubuntu-desktop".to_string()]
            );
        });
    }

    #[test]
    fn effective_sources_seeds_default_when_empty() {
        with_temp_root(|| {
            let s = sources();
            assert_eq!(s.len(), 1);
            assert_eq!(s[0].registry, DEFAULT_SOURCE_REGISTRY);
        });
    }

    #[test]
    fn add_source_materializes_default_then_appends() {
        with_temp_root(|| {
            assert!(add_source("ghcr.io/pallewela").unwrap());
            let s = sources();
            assert_eq!(s.len(), 2);
            assert_eq!(s[0].registry, DEFAULT_SOURCE_REGISTRY);
            assert_eq!(s[1].registry, "ghcr.io/pallewela");
        });
    }

    #[test]
    fn add_source_is_idempotent() {
        with_temp_root(|| {
            assert!(add_source("ghcr.io/pallewela").unwrap());
            assert!(!add_source("ghcr.io/pallewela").unwrap());
            assert_eq!(
                sources()
                    .iter()
                    .filter(|s| s.registry == "ghcr.io/pallewela")
                    .count(),
                1
            );
        });
    }

    #[test]
    fn remove_source_and_reseed_when_last() {
        with_temp_root(|| {
            add_source("ghcr.io/pallewela").unwrap();
            assert!(remove_source("ghcr.io/pallewela").unwrap());
            assert_eq!(sources().len(), 1);
            // Removing the last remaining source re-seeds the default.
            assert!(remove_source(DEFAULT_SOURCE_REGISTRY).unwrap());
            let s = sources();
            assert_eq!(s.len(), 1);
            assert_eq!(s[0].registry, DEFAULT_SOURCE_REGISTRY);
            // Removing something absent is a no-op false.
            assert!(!remove_source("ghcr.io/nope").unwrap());
        });
    }

    #[test]
    fn move_source_changes_priority() {
        with_temp_root(|| {
            add_source("ghcr.io/pallewela").unwrap();
            add_source("ghcr.io/third").unwrap();
            // [cirruslabs, pallewela, third] -> move third to position 1
            assert!(move_source("ghcr.io/third", 1).unwrap());
            let regs: Vec<_> = sources().into_iter().map(|s| s.registry).collect();
            assert_eq!(
                regs,
                vec!["ghcr.io/third", "ghcr.io/cirruslabs", "ghcr.io/pallewela"]
            );
            // Out-of-range position clamps to the end.
            assert!(move_source("ghcr.io/third", 99).unwrap());
            let regs: Vec<_> = sources().into_iter().map(|s| s.registry).collect();
            assert_eq!(regs.last().unwrap(), "ghcr.io/third");
            assert!(!move_source("ghcr.io/absent", 1).unwrap());
        });
    }

    #[test]
    fn old_schema_without_sources_still_loads() {
        with_temp_root(|| {
            paths::ensure_dirs().unwrap();
            std::fs::write(paths::state_file(), b"default_vm = \"hello\"\n").unwrap();
            let s = State::load();
            assert!(s.sources.is_empty());
            assert_eq!(sources().len(), 1);
        });
    }
}
