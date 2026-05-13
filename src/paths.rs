use std::path::PathBuf;

use crate::runtime;

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).expect("HOME is not set")
}

pub fn state_dir() -> PathBuf {
    runtime::state_root_override().unwrap_or_else(|| home().join(".local/share/rusta"))
}

pub fn state_file() -> PathBuf {
    state_dir().join("state.toml")
}

pub fn provision_dir() -> PathBuf {
    state_dir().join("provision")
}

pub fn provision_script(vm: &str) -> PathBuf {
    provision_dir().join(format!("{vm}.sh"))
}

pub fn run_dir() -> PathBuf {
    state_dir().join("run")
}

pub fn pid_file(vm: &str) -> PathBuf {
    run_dir().join(format!("{vm}.pid"))
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(state_dir())?;
    std::fs::create_dir_all(provision_dir())?;
    std::fs::create_dir_all(run_dir())?;
    Ok(())
}

pub fn ssh_dir() -> PathBuf {
    runtime::ssh_dir_override().unwrap_or_else(|| home().join(".ssh"))
}

pub fn ssh_config() -> PathBuf {
    ssh_dir().join("config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_compose_from_state_dir() {
        let s = state_dir();
        assert_eq!(state_file(), s.join("state.toml"));
        assert_eq!(provision_dir(), s.join("provision"));
        assert_eq!(provision_script("vm"), s.join("provision/vm.sh"));
        assert_eq!(run_dir(), s.join("run"));
        assert_eq!(pid_file("vm"), s.join("run/vm.pid"));
        assert_eq!(ssh_config(), ssh_dir().join("config"));
    }
}
