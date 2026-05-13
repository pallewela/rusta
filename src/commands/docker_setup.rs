use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::cli::VmOnlyArgs;
use crate::commands::{ensure_docker_cli, ensure_sshpass};
use crate::error::{Error, Result};
use crate::io as rio;
use crate::paths;
use crate::picker;
use crate::runtime::bin_for;
use crate::ssh;
use crate::tart;

const VM_USER: &str = "admin";
const VM_PASSWORD: &str = "admin";

pub fn run(args: VmOnlyArgs) -> Result<u8> {
    let vm = picker::resolve_vm(args.vm)?;
    if !tart::exists(&vm)? {
        return Err(Error::not_found(format!("VM '{vm}' not found")));
    }
    ensure_sshpass()?;
    ensure_docker_cli()?;

    let started_by_us = crate::commands::ssh_copy::ensure_running(&vm)?;
    let ip = tart::wait_for_ip(&vm, Duration::from_secs(60))?;
    ssh::wait_for_ssh(VM_USER, VM_PASSWORD, &ip, Duration::from_secs(120))?;

    let key = ensure_host_ed25519_key()?;
    rio::info("Copying SSH public key to guest...");
    ssh::ssh_copy_id(VM_USER, VM_PASSWORD, &ip, &key.with_extension("pub"))?;
    rio::ok("Public key installed in guest");

    let install = r#"set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

if command -v docker >/dev/null 2>&1; then
  echo ">>> Docker already installed: $(docker --version)"
else
  echo ">>> Installing Docker via official convenience script..."
  curl -fsSL https://get.docker.com | sudo sh
fi

if ! id -nG "$USER" | grep -qw docker; then
  echo ">>> Adding $USER to docker group..."
  sudo usermod -aG docker "$USER"
fi

sudo systemctl enable --now docker
echo ">>> Docker ready: $(docker --version)"
"#;
    rio::info(&format!("Installing Docker Engine in guest '{vm}'..."));
    ssh::ssh_with_stdin(VM_USER, VM_PASSWORD, &ip, &["bash", "-s"], install.as_bytes())?;
    rio::ok("Docker Engine installed and running in guest");

    let host_alias = format!("docker-{vm}");
    write_ssh_config_block(&host_alias, &ip, VM_USER, &key)?;
    create_docker_context(&host_alias, VM_USER)?;

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

    print_summary(&vm, &host_alias, &ip);
    Ok(0)
}

fn ensure_host_ed25519_key() -> Result<PathBuf> {
    let key = paths::ssh_dir().join("id_ed25519");
    if key.exists() {
        return Ok(key);
    }
    std::fs::create_dir_all(paths::ssh_dir()).map_err(|e| Error::msg(e.to_string()))?;
    rio::info("Generating SSH key pair (ed25519)...");
    let status = Command::new(bin_for("ssh-keygen"))
        .args(["-t", "ed25519", "-f"])
        .arg(&key)
        .args(["-N", "", "-q"])
        .status()
        .map_err(|e| Error::cmd("ssh-keygen", e))?;
    if !status.success() {
        return Err(Error::msg("ssh-keygen failed".to_string()));
    }
    rio::ok(&format!("SSH key created: {}", key.display()));
    Ok(key)
}

fn write_ssh_config_block(alias: &str, ip: &str, user: &str, key: &std::path::Path) -> Result<()> {
    use std::io::Write;
    let cfg = paths::ssh_config();
    std::fs::create_dir_all(paths::ssh_dir()).map_err(|e| Error::msg(e.to_string()))?;
    // chmod 700 ~/.ssh
    let _ = set_mode(&paths::ssh_dir(), 0o700);

    let existing = std::fs::read_to_string(&cfg).unwrap_or_default();
    let needle = format!("Host {alias}");
    if existing.lines().any(|l| l.trim() == needle.as_str()) {
        rio::skip(&format!(
            "SSH config entry '{alias}' already exists in {}",
            cfg.display()
        ));
        return Ok(());
    }
    rio::info(&format!(
        "Adding SSH config entry '{alias}' to {}...",
        cfg.display()
    ));
    let block = format!(
        r#"
# Docker VM: {alias} (added by rusta docker-setup)
Host {alias}
    HostName {ip}
    User {user}
    IdentityFile {keyp}
    IdentitiesOnly yes
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
"#,
        alias = alias,
        ip = ip,
        user = user,
        keyp = key.display()
    );
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&cfg)
        .map_err(|e| Error::msg(e.to_string()))?;
    f.write_all(block.as_bytes()).map_err(|e| Error::msg(e.to_string()))?;
    let _ = set_mode(&cfg, 0o600);
    rio::ok(&format!("SSH config entry added: Host {alias} -> {ip}"));
    Ok(())
}

fn create_docker_context(alias: &str, user: &str) -> Result<()> {
    let inspect = Command::new(bin_for("docker"))
        .args(["context", "inspect", alias])
        .output();
    if matches!(inspect, Ok(o) if o.status.success()) {
        rio::skip(&format!("Docker context '{alias}' already exists"));
        return Ok(());
    }
    rio::info(&format!("Creating Docker context '{alias}'..."));
    let status = Command::new(bin_for("docker"))
        .args([
            "context",
            "create",
            alias,
            "--docker",
            &format!("host=ssh://{user}@{alias}"),
        ])
        .status()
        .map_err(|e| Error::cmd("docker context create", e))?;
    if !status.success() {
        return Err(Error::msg("`docker context create` failed".to_string()));
    }
    rio::ok(&format!("Docker context created: {alias}"));
    Ok(())
}

fn set_mode(path: &std::path::Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let p = std::fs::metadata(path)?.permissions();
    let mut p = p;
    p.set_mode(mode);
    std::fs::set_permissions(path, p)
}

fn print_summary(vm: &str, alias: &str, ip: &str) {
    println!();
    println!("==> Docker setup complete!");
    println!();
    println!("  VM name        : {vm}");
    println!("  SSH alias      : {alias}");
    println!("  Docker context : {alias}");
    println!();
    println!("  Usage:");
    println!("    1. Start the VM:   rusta up {vm}");
    println!("    2. Switch context: docker context use {alias}");
    println!("    3. Use Docker:     docker ps");
    println!();
    println!("  Note: the SSH config uses a fixed IP ({ip}).");
    println!("  If the VM gets a new IP after reboot, update HostName in ~/.ssh/config.");
}
