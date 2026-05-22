use std::io::Write;
use std::process::Command;

use crate::cli::{Cli, Command as Cmd};
use crate::error::{Error, Result};
use crate::io as rio;
use crate::runtime::{bin_for, skip_preflight};

mod completions;
mod create;
mod default_cmd;
mod delete;
mod docker_setup;
mod down;
mod ip;
mod list;
mod set_gui;
mod ssh_cmd;
mod ssh_copy;
mod up;
mod versions;

pub fn dispatch(cli: Cli) -> Result<u8> {
    let Some(command) = cli.command else {
        // No subcommand: equivalent to --help, exit 0.
        print_top_help();
        return Ok(0);
    };

    // Most commands require Apple Silicon + brew + tart. A few are
    // host-independent: `versions` only needs network; `completions` and
    // `man` are packaging plumbing that must run anywhere.
    match &command {
        Cmd::Versions | Cmd::Completions(_) | Cmd::Man(_) => {}
        _ => preflight()?,
    }

    match command {
        Cmd::Up(a) => up::run(a),
        Cmd::Down(a) => down::run(a),
        Cmd::Create(a) => create::run(a),
        Cmd::Delete(a) => delete::run(a),
        Cmd::List => list::run(),
        Cmd::Versions => versions::run(),
        Cmd::Default(a) => default_cmd::run(a),
        Cmd::Ip(a) => ip::run(a),
        Cmd::Ssh(a) => ssh_cmd::run(a),
        Cmd::DockerSetup(a) => docker_setup::run(a),
        Cmd::SshCopy(a) => ssh_copy::run(a),
        Cmd::SetGui(a) => set_gui::run(a),
        Cmd::Completions(a) => completions::completions(a),
        Cmd::Man(a) => completions::man(a),
    }
}

fn print_top_help() {
    use clap::CommandFactory;
    let mut cmd = Cli::command();
    let _ = cmd.print_help();
    println!();
}

fn preflight() -> Result<()> {
    if skip_preflight() {
        crate::paths::ensure_dirs().map_err(|e| Error::msg(format!("create state dirs: {e}")))?;
        return Ok(());
    }
    let arch = uname_m();
    if arch != "arm64" {
        return Err(Error::msg(format!(
            "rusta requires Apple Silicon (arm64). Detected: {arch}"
        )));
    }
    ensure_brew()?;
    ensure_tart()?;
    crate::paths::ensure_dirs().map_err(|e| Error::msg(format!("create state dirs: {e}")))?;
    Ok(())
}

fn uname_m() -> String {
    Command::new(bin_for("uname"))
        .arg("-m")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub fn which(name: &str) -> bool {
    Command::new("/usr/bin/env")
        .arg("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", bin_for(name)))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn ensure_brew() -> Result<()> {
    if !which("brew") {
        return Err(Error::msg(
            "Homebrew is required. Install it from https://brew.sh first.".to_string(),
        ));
    }
    Ok(())
}

fn ensure_tart() -> Result<()> {
    if which("tart") {
        if let Some(v) = crate::tart::version() {
            // quietly note in --verbose
            if rio::verbose() {
                rio::skip(&format!("tart already installed ({v})"));
            }
        }
        return Ok(());
    }
    rio::info("Installing tart (Apple Virtualization CLI)...");
    let status = Command::new(bin_for("brew"))
        .args(["install", "cirruslabs/cli/tart"])
        .status()
        .map_err(|e| Error::cmd("brew install tart", e))?;
    if !status.success() {
        return Err(Error::msg("`brew install cirruslabs/cli/tart` failed".to_string()));
    }
    rio::ok("tart installed");
    Ok(())
}

pub fn ensure_sshpass() -> Result<()> {
    if which("sshpass") {
        return Ok(());
    }
    rio::info("Installing sshpass...");
    let status = Command::new(bin_for("brew"))
        .args(["install", "sshpass"])
        .status()
        .map_err(|e| Error::cmd("brew install sshpass", e))?;
    if !status.success() {
        return Err(Error::msg("`brew install sshpass` failed".to_string()));
    }
    rio::ok("sshpass installed");
    Ok(())
}

pub fn ensure_docker_cli() -> Result<()> {
    if which("docker") {
        return Ok(());
    }
    rio::info("Installing Docker CLI on host...");
    let status = Command::new(bin_for("brew"))
        .args(["install", "docker"])
        .status()
        .map_err(|e| Error::cmd("brew install docker", e))?;
    if !status.success() {
        return Err(Error::msg("`brew install docker` failed".to_string()));
    }
    rio::ok("Docker CLI installed");
    Ok(())
}

/// Confirm interactively. Returns Ok(true) on yes.
pub fn confirm(prompt: &str) -> Result<bool> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return Err(Error::msg(format!(
            "{prompt} (stdin is not a TTY; pass --yes to confirm non-interactively)"
        )));
    }
    print!("{prompt} [y/N]: ");
    std::io::stdout().flush().ok();
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).map_err(|e| Error::msg(e.to_string()))?;
    Ok(matches!(buf.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}
