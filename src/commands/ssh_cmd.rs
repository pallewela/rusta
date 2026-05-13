use std::time::Duration;

use crate::cli::SshArgs;
use crate::commands::ensure_sshpass;
use crate::error::{Error, Result};
use crate::io as rio;
use crate::picker;
use crate::ssh;
use crate::tart;

// SSH default credentials match the bash script: admin/admin.
// These are not surfaced as `ssh` subcommand flags by design — set them via the
// VM image at create time and use the docker-setup path for key-based auth.
const DEFAULT_USER: &str = "admin";
const DEFAULT_PASSWORD: &str = "admin";

pub fn run(args: SshArgs) -> Result<u8> {
    let vm = picker::resolve_vm(args.vm)?;
    if !tart::exists(&vm)? {
        return Err(Error::not_found(format!("VM '{vm}' not found")));
    }
    if !tart::is_running(&vm)? {
        if !args.auto_up {
            return Err(Error::msg(format!(
                "VM '{vm}' is not running. Run `rusta up {vm}` first, or pass --auto-up."
            )));
        }
        rio::info(&format!("Auto-booting VM '{vm}'..."));
        crate::commands::up::run(crate::cli::UpArgs {
            vm: Some(vm.clone()),
            graphical: false,
            no_gui: false,
        })?;
    }

    ensure_sshpass()?;

    let ip = tart::wait_for_ip(&vm, Duration::from_secs(60))?;
    ssh::wait_for_ssh(DEFAULT_USER, DEFAULT_PASSWORD, &ip, Duration::from_secs(120))?;

    let code = ssh::ssh_interactive(DEFAULT_USER, DEFAULT_PASSWORD, &ip, &args.remote)?;
    Ok(code.try_into().unwrap_or(1))
}
