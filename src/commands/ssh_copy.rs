use std::path::PathBuf;
use std::time::Duration;

use crate::cli::VmOnlyArgs;
use crate::commands::ensure_sshpass;
use crate::error::{Error, Result};
use crate::io as rio;
use crate::paths;
use crate::picker;
use crate::ssh;
use crate::tart;

const VM_USER: &str = "admin";
const VM_PASSWORD: &str = "admin";

pub fn run(args: VmOnlyArgs) -> Result<u8> {
    let vm = picker::resolve_vm(args.vm)?;
    if !tart::exists(&vm)? {
        return Err(Error::not_found(format!("VM '{vm}' not found")));
    }
    let host_ssh = paths::ssh_dir();
    if !host_ssh.exists() {
        return Err(Error::msg(format!(
            "Host has no {}; nothing to copy.",
            host_ssh.display()
        )));
    }

    let items = collect_keys(&host_ssh)?;
    if items.is_empty() {
        rio::skip(&format!(
            "No SSH keys (id_*, *.pem) found under {}; nothing to copy.",
            host_ssh.display()
        ));
        return Ok(0);
    }

    ensure_sshpass()?;

    let started_by_us = ensure_running(&vm)?;
    let ip = tart::wait_for_ip(&vm, Duration::from_secs(60))?;
    ssh::wait_for_ssh(VM_USER, VM_PASSWORD, &ip, Duration::from_secs(120))?;

    rio::info(&format!(
        "Copying {} SSH file(s) from {} to guest /home/{}/.ssh/...",
        items.len(),
        host_ssh.display(),
        VM_USER
    ));
    ssh::ssh_run(VM_USER, VM_PASSWORD, &ip, &["mkdir -p ~/.ssh && chmod 700 ~/.ssh"])?;
    let refs: Vec<&std::path::Path> = items.iter().map(|p| p.as_path()).collect();
    ssh::scp_files(VM_USER, VM_PASSWORD, &ip, &refs, ".ssh/")?;

    let fix_perm = r#"set -euo pipefail
cd ~/.ssh
chmod 700 .
for f in id_* *.pem; do
  [ -f "$f" ] || continue
  case "$f" in
    *.pub) chmod 644 "$f" ;;
    *)     chmod 600 "$f" ;;
  esac
done
"#;
    ssh::ssh_with_stdin(VM_USER, VM_PASSWORD, &ip, &["bash", "-s"], fix_perm.as_bytes())?;
    rio::ok(&format!("SSH configuration copied to guest '{vm}'"));

    if started_by_us {
        rio::info("Shutting down the guest (started by rusta)...");
        let _ = tart::exec_quiet(&vm, &["sudo", "shutdown", "-h", "now"]);
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while std::time::Instant::now() < deadline {
            if !tart::is_running(&vm)? {
                tart::remove_pid_file(&vm);
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(0)
}

fn collect_keys(dir: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| Error::msg(e.to_string()))? {
        let e = entry.map_err(|e| Error::msg(e.to_string()))?;
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let name = match p.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with("id_") || name.ends_with(".pem") {
            out.push(p);
        }
    }
    Ok(out)
}

pub fn ensure_running(vm: &str) -> Result<bool> {
    if tart::is_running(vm)? {
        rio::info(&format!("VM '{vm}' is already running; waiting for IP..."));
        return Ok(false);
    }
    rio::info(&format!("Starting VM '{vm}' headlessly..."));
    let child = tart::run_background(vm, true)?;
    let _ = tart::write_pid_file(vm, child.id());
    std::mem::forget(child);
    tart::wait_for_guest_agent(vm, Duration::from_secs(120))?;
    Ok(true)
}
