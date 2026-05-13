use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::io as rio;
use crate::runtime::bin_for;

pub fn ssh_opts() -> Vec<&'static str> {
    let mut v = vec![
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        "PubkeyAuthentication=no",
        "-o",
        if rio::verbose() { "LogLevel=INFO" } else { "LogLevel=ERROR" },
        "-o",
        "ConnectTimeout=10",
        "-o",
        "ServerAliveInterval=30",
        "-o",
        "ServerAliveCountMax=120",
    ];
    // ssh wants -o flags; nothing else here.
    v.shrink_to_fit();
    v
}

/// Run an SSH command with password auth via sshpass.
pub fn ssh_run(user: &str, password: &str, host: &str, remote: &[&str]) -> Result<()> {
    let mut cmd = Command::new(bin_for("sshpass"));
    cmd.arg("-p").arg(password).arg("ssh");
    for o in ssh_opts() {
        cmd.arg(o);
    }
    cmd.arg(format!("{user}@{host}"));
    for a in remote {
        cmd.arg(a);
    }
    let status = cmd.status().map_err(|e| Error::cmd("ssh", e))?;
    if !status.success() {
        return Err(Error::msg("ssh command failed".to_string()));
    }
    Ok(())
}

pub fn ssh_run_quiet(user: &str, password: &str, host: &str, remote: &[&str]) -> bool {
    let mut cmd = Command::new(bin_for("sshpass"));
    cmd.arg("-p").arg(password).arg("ssh");
    for o in ssh_opts() {
        cmd.arg(o);
    }
    cmd.arg(format!("{user}@{host}"));
    for a in remote {
        cmd.arg(a);
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    matches!(cmd.status(), Ok(s) if s.success())
}

pub fn ssh_with_stdin(user: &str, password: &str, host: &str, remote: &[&str], stdin: &[u8]) -> Result<()> {
    let mut cmd = Command::new(bin_for("sshpass"));
    cmd.arg("-p").arg(password).arg("ssh");
    for o in ssh_opts() {
        cmd.arg(o);
    }
    cmd.arg(format!("{user}@{host}"));
    for a in remote {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| Error::cmd("ssh", e))?;
    if let Some(mut s) = child.stdin.take() {
        s.write_all(stdin).map_err(|e| Error::cmd("ssh stdin", e))?;
    }
    let st = child.wait().map_err(|e| Error::cmd("ssh wait", e))?;
    if !st.success() {
        return Err(Error::msg("ssh command failed".to_string()));
    }
    Ok(())
}

pub fn scp_files(user: &str, password: &str, host: &str, files: &[&Path], remote_dest: &str) -> Result<()> {
    let mut cmd = Command::new(bin_for("sshpass"));
    cmd.arg("-p").arg(password).arg("scp");
    for o in ssh_opts() {
        cmd.arg(o);
    }
    for f in files {
        cmd.arg(f);
    }
    cmd.arg(format!("{user}@{host}:{remote_dest}"));
    let status = cmd.status().map_err(|e| Error::cmd("scp", e))?;
    if !status.success() {
        return Err(Error::msg("scp failed".to_string()));
    }
    Ok(())
}

pub fn ssh_copy_id(user: &str, password: &str, host: &str, pubkey: &Path) -> Result<()> {
    let mut cmd = Command::new(bin_for("sshpass"));
    cmd.arg("-p").arg(password).arg("ssh-copy-id").arg("-i").arg(pubkey);
    for o in ssh_opts() {
        cmd.arg(o);
    }
    cmd.arg(format!("{user}@{host}"));
    let status = cmd.status().map_err(|e| Error::cmd("ssh-copy-id", e))?;
    if !status.success() {
        return Err(Error::msg("ssh-copy-id failed".to_string()));
    }
    Ok(())
}

/// Open an interactive SSH session (no exit-on-fail capture).
pub fn ssh_interactive(user: &str, password: &str, host: &str, remote: &[String]) -> Result<i32> {
    let mut cmd = Command::new(bin_for("sshpass"));
    cmd.arg("-p").arg(password).arg("ssh");
    for o in ssh_opts() {
        cmd.arg(o);
    }
    cmd.arg(format!("{user}@{host}"));
    for a in remote {
        cmd.arg(a);
    }
    let status = cmd.status().map_err(|e| Error::cmd("ssh", e))?;
    Ok(status.code().unwrap_or(1))
}

/// Wait until SSH is accepting connections (password auth).
pub fn wait_for_ssh(user: &str, password: &str, host: &str, timeout: Duration) -> Result<()> {
    let timeout = cap_timeout(timeout);
    let start = Instant::now();
    let mut first = true;
    loop {
        if ssh_run_quiet(user, password, host, &["true"]) {
            return Ok(());
        }
        if !first && start.elapsed() >= timeout {
            break;
        }
        first = false;
        std::thread::sleep(ssh_poll_interval());
    }
    Err(Error::msg(format!("ssh on {host} did not become ready in time")))
}

fn cap_timeout(t: Duration) -> Duration {
    if let Ok(s) = std::env::var("RUSTA_MAX_TIMEOUT_S") {
        if let Ok(n) = s.parse::<u64>() {
            return t.min(Duration::from_secs(n));
        }
    }
    t
}

fn ssh_poll_interval() -> Duration {
    if let Ok(ms) = std::env::var("RUSTA_POLL_MS") {
        if let Ok(n) = ms.parse::<u64>() {
            return Duration::from_millis(n);
        }
    }
    Duration::from_secs(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_opts_includes_required_flags() {
        let opts = ssh_opts();
        let joined: Vec<String> = opts.iter().map(|s| s.to_string()).collect();
        let s = joined.join(" ");
        assert!(s.contains("StrictHostKeyChecking=no"));
        assert!(s.contains("UserKnownHostsFile=/dev/null"));
        assert!(s.contains("PubkeyAuthentication=no"));
        assert!(s.contains("ConnectTimeout=10"));
    }

    #[test]
    fn ssh_opts_loglevel_changes_with_verbose() {
        let prev = rio::verbose();
        rio::set_verbose(false);
        assert!(ssh_opts().iter().any(|s| *s == "LogLevel=ERROR"));
        rio::set_verbose(true);
        assert!(ssh_opts().iter().any(|s| *s == "LogLevel=INFO"));
        rio::set_verbose(prev);
    }

    #[test]
    fn ssh_poll_interval_honors_env_var() {
        let prev = std::env::var("RUSTA_POLL_MS").ok();
        std::env::set_var("RUSTA_POLL_MS", "42");
        assert_eq!(ssh_poll_interval(), Duration::from_millis(42));
        std::env::set_var("RUSTA_POLL_MS", "not-a-number");
        assert_eq!(ssh_poll_interval(), Duration::from_secs(3));
        match prev {
            Some(v) => std::env::set_var("RUSTA_POLL_MS", v),
            None => std::env::remove_var("RUSTA_POLL_MS"),
        }
    }
}

