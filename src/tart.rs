use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::runtime::bin_for;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vm {
    pub name: String,
    pub status: String,
}

fn tart_cmd() -> Command {
    Command::new(bin_for("tart"))
}

/// Pure parser for `tart list --format json` output. Filters to `Source=="local"`.
pub fn parse_list_json(bytes: &[u8]) -> Result<Vec<Vm>> {
    let parsed: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| Error::msg(format!("parse `tart list --format json`: {e}")))?;
    let arr = parsed
        .as_array()
        .ok_or_else(|| Error::msg("`tart list --format json` did not return an array".to_string()))?;
    let mut vms = Vec::new();
    for item in arr {
        let source = item.get("Source").and_then(|v| v.as_str()).unwrap_or("");
        if source != "local" {
            continue;
        }
        let name = item
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let status = item
            .get("State")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() {
            continue;
        }
        vms.push(Vm { name, status });
    }
    Ok(vms)
}

pub fn list() -> Result<Vec<Vm>> {
    let out = tart_cmd()
        .args(["list", "--format", "json"])
        .output()
        .map_err(|e| Error::cmd("tart list", e))?;
    if !out.status.success() {
        return Err(Error::msg(format!(
            "`tart list` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    parse_list_json(&out.stdout)
}

pub fn exists(vm: &str) -> Result<bool> {
    Ok(list()?.iter().any(|v| v.name == vm))
}

pub fn is_running(vm: &str) -> Result<bool> {
    Ok(list()?
        .iter()
        .any(|v| v.name == vm && v.status.eq_ignore_ascii_case("running")))
}

pub fn clone_image(image: &str, vm: &str) -> Result<()> {
    run_inherit(&["clone", image, vm])
}

pub fn set_resources(vm: &str, cpus: u32, mem_mb: u32, disk_gb: u32) -> Result<()> {
    run_inherit(&[
        "set",
        vm,
        "--cpu",
        &cpus.to_string(),
        "--memory",
        &mem_mb.to_string(),
        "--disk-size",
        &disk_gb.to_string(),
    ])
}

pub fn delete(vm: &str) -> Result<()> {
    run_inherit(&["delete", vm])
}

pub fn stop(vm: &str) -> Result<()> {
    run_inherit(&["stop", vm])
}

pub fn ip(vm: &str) -> Result<String> {
    let out = tart_cmd()
        .args(["ip", vm])
        .output()
        .map_err(|e| Error::cmd("tart ip", e))?;
    if !out.status.success() {
        return Err(Error::msg(format!(
            "`tart ip {vm}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn wait_for_ip(vm: &str, timeout: Duration) -> Result<String> {
    let timeout = cap_timeout(timeout);
    let start = Instant::now();
    let mut first = true;
    loop {
        if let Ok(ip) = ip(vm) {
            if !ip.is_empty() {
                return Ok(ip);
            }
        }
        if !first && start.elapsed() >= timeout {
            break;
        }
        first = false;
        std::thread::sleep(poll_interval());
    }
    Err(Error::msg(format!("timed out waiting for IP of '{vm}'")))
}

pub fn wait_for_guest_agent(vm: &str, timeout: Duration) -> Result<()> {
    let timeout = cap_timeout(timeout);
    let start = Instant::now();
    let mut first = true;
    loop {
        let status = tart_cmd()
            .args(["exec", vm, "true"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                return Ok(());
            }
        }
        if !first && start.elapsed() >= timeout {
            break;
        }
        first = false;
        std::thread::sleep(poll_interval());
    }
    Err(Error::msg(format!(
        "timed out waiting for tart guest agent on '{vm}'"
    )))
}

pub fn exec(vm: &str, cmd: &[&str]) -> Result<()> {
    let mut args = vec!["exec", vm];
    args.extend_from_slice(cmd);
    run_inherit(&args)
}

pub fn exec_quiet(vm: &str, cmd: &[&str]) -> Result<()> {
    let mut args = vec!["exec", vm];
    args.extend_from_slice(cmd);
    let status = tart_cmd()
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| Error::cmd("tart exec", e))?;
    if !status.success() {
        return Err(Error::msg(format!("`tart {}` failed", args.join(" "))));
    }
    Ok(())
}

pub fn exec_with_stdin(vm: &str, cmd: &[&str], stdin: &[u8]) -> Result<()> {
    let mut args = vec!["exec", "-i", vm];
    args.extend_from_slice(cmd);
    let mut child = tart_cmd()
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| Error::cmd("tart exec", e))?;
    if let Some(mut s) = child.stdin.take() {
        s.write_all(stdin).map_err(|e| Error::cmd("tart exec stdin", e))?;
    }
    let st = child.wait().map_err(|e| Error::cmd("tart exec wait", e))?;
    if !st.success() {
        return Err(Error::msg(format!("`tart {}` failed", args.join(" "))));
    }
    Ok(())
}

pub fn run_background(vm: &str, headless: bool) -> Result<Child> {
    let mut args = vec!["run".to_string(), vm.to_string()];
    if headless {
        args.push("--no-graphics".to_string());
    }
    tart_cmd()
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::cmd("tart run", e))
}

pub fn version() -> Option<String> {
    let out = tart_cmd().arg("--version").output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn run_inherit(args: &[&str]) -> Result<()> {
    let status = tart_cmd()
        .args(args)
        .status()
        .map_err(|e| Error::cmd("tart", e))?;
    if !status.success() {
        return Err(Error::msg(format!("`tart {}` failed", args.join(" "))));
    }
    Ok(())
}

pub fn write_pid_file(vm: &str, pid: u32) -> std::io::Result<()> {
    crate::paths::ensure_dirs()?;
    let path = crate::paths::pid_file(vm);
    let mut f = std::fs::File::create(path)?;
    write!(f, "{pid}")
}

pub fn read_pid_file(vm: &str) -> Option<u32> {
    let p = crate::paths::pid_file(vm);
    let s = std::fs::read_to_string(p).ok()?;
    s.trim().parse().ok()
}

pub fn remove_pid_file(vm: &str) {
    let _ = std::fs::remove_file(crate::paths::pid_file(vm));
}

pub fn pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

pub fn kill_pid(pid: u32) {
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
    }
}

/// Poll interval can be shortened in tests via `RUSTA_POLL_MS`.
fn poll_interval() -> Duration {
    if let Ok(ms) = std::env::var("RUSTA_POLL_MS") {
        if let Ok(n) = ms.parse::<u64>() {
            return Duration::from_millis(n);
        }
    }
    Duration::from_secs(2)
}

/// Cap any timeout the caller asked for at `RUSTA_MAX_TIMEOUT_S` seconds when set
/// (test hook so we don't have to wait the full production timeout).
fn cap_timeout(t: Duration) -> Duration {
    if let Ok(s) = std::env::var("RUSTA_MAX_TIMEOUT_S") {
        if let Ok(n) = s.parse::<u64>() {
            return t.min(Duration::from_secs(n));
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_list_filters_to_local_source() {
        let json = br#"[
            {"Source":"local","Name":"vm-a","State":"stopped"},
            {"Source":"oci","Name":"ghcr.io/x:1","State":"stopped"},
            {"Source":"local","Name":"vm-b","State":"running"}
        ]"#;
        let vms = parse_list_json(json).unwrap();
        assert_eq!(
            vms,
            vec![
                Vm { name: "vm-a".into(), status: "stopped".into() },
                Vm { name: "vm-b".into(), status: "running".into() },
            ]
        );
    }

    #[test]
    fn parse_list_rejects_non_array() {
        let err = parse_list_json(b"{}").unwrap_err();
        assert!(err.message.contains("array"));
    }

    #[test]
    fn parse_list_rejects_invalid_json() {
        let err = parse_list_json(b"not-json").unwrap_err();
        assert!(err.message.contains("parse"));
    }

    #[test]
    fn parse_list_skips_empty_name() {
        let json = br#"[
            {"Source":"local","Name":"","State":"stopped"},
            {"Source":"local","Name":"keep","State":"stopped"}
        ]"#;
        assert_eq!(parse_list_json(json).unwrap().len(), 1);
    }

    #[test]
    fn pid_alive_self_is_alive_and_zero_is_invalid() {
        let me = std::process::id();
        assert!(pid_alive(me));
        // PID 0 is reserved and not a real process; kill(0, 0) signals the process group.
        // We just make sure pid_alive doesn't panic. Behavior of 0 is unspecified, so we
        // only assert non-existent high pid is reported dead.
        assert!(!pid_alive(2_000_000_001));
    }
}
