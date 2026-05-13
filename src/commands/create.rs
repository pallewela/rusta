use std::time::Duration;

use crate::cli::CreateArgs;
use crate::error::{Error, Result};
use crate::io as rio;
use crate::paths;
use crate::provision;
use crate::tart;

pub fn run(args: CreateArgs) -> Result<u8> {
    validate_name_opt(args.vm.as_deref())?;
    if let Some(gui) = args.gui.as_deref() {
        if provision::display_manager_for(gui).is_none() {
            return Err(Error::msg(format!(
                "--gui accepts: ubuntu-desktop, xubuntu-desktop, lubuntu-desktop, lightdm (got '{gui}')"
            )));
        }
    }

    let vm_name = args
        .vm
        .clone()
        .unwrap_or_else(|| format!("ubuntu-{}", args.version.replace('.', "")));

    let variant = if args.gui.is_some() { "desktop" } else { "server" };
    println!();
    rio::info(&format!(
        "Ubuntu {} — {} — Tart OCI (ghcr.io/cirruslabs/ubuntu)",
        args.version, variant
    ));
    println!();

    if tart::exists(&vm_name)? {
        rio::skip(&format!("VM '{vm_name}' already exists"));
        let mut hint = format!(
            "rusta delete {vm_name} && rusta create {vm_name} --version {}",
            args.version
        );
        if let Some(gui) = args.gui.as_deref() {
            hint.push_str(&format!(" --gui {gui}"));
        }
        rio::info(&format!("To recreate: {hint}"));
        return Ok(0);
    }

    rio::info(&format!(
        "Cloning Ubuntu {} from ghcr.io/cirruslabs/ubuntu:{}...",
        args.version, args.version
    ));
    let image = format!("ghcr.io/cirruslabs/ubuntu:{}", args.version);
    tart::clone_image(&image, &vm_name)?;
    tart::set_resources(&vm_name, args.cpus, args.memory, args.disk)?;
    rio::ok(&format!(
        "VM created: {} ({} CPUs, {} GB RAM, {} GB disk)",
        vm_name,
        args.cpus,
        args.memory / 1024,
        args.disk
    ));

    // Generate and persist provisioning script for debuggability.
    let script = provision::generate(&provision::Spec {
        ubuntu_version: &args.version,
        gui: args.gui.as_deref(),
    });
    paths::ensure_dirs().map_err(|e| Error::msg(e.to_string()))?;
    let script_path = paths::provision_script(&vm_name);
    std::fs::write(&script_path, &script).map_err(|e| Error::msg(e.to_string()))?;
    rio::ok(&format!("Provisioning script: {}", script_path.display()));

    // Boot, wait for guest agent, upload + run provisioning, shut down.
    let headless = !args.debug_no_headless;
    if headless {
        rio::info(&format!("Starting VM '{vm_name}' headlessly..."));
    } else {
        rio::info(&format!("Starting VM '{vm_name}' with graphics window (debug)..."));
    }
    let child = tart::run_background(&vm_name, headless)?;
    let pid = child.id();
    let _ = tart::write_pid_file(&vm_name, pid);
    std::mem::forget(child);

    let cleanup = ProcessGuard { pid };

    rio::info("Waiting for guest agent...");
    tart::wait_for_guest_agent(&vm_name, Duration::from_secs(120))?;
    rio::ok("Guest agent is ready");

    rio::info("Uploading provisioning script to guest...");
    tart::exec_with_stdin(
        &vm_name,
        &["bash", "-c", "cat > /tmp/provision.sh && chmod +x /tmp/provision.sh"],
        script.as_bytes(),
    )?;

    rio::info("Running provisioning inside the guest (this may take a while)...");
    tart::exec(&vm_name, &["bash", "/tmp/provision.sh"])?;
    rio::ok("Provisioning complete!");

    rio::info("Shutting down the guest...");
    let _ = tart::exec_quiet(&vm_name, &["sudo", "shutdown", "-h", "now"]);
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        if !tart::is_running(&vm_name)? {
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    tart::remove_pid_file(&vm_name);
    rio::ok("VM stopped after provisioning");
    drop(cleanup);

    if args.ssh_copy_keys {
        println!();
        rio::info(&format!("Copying host SSH configuration into '{vm_name}'..."));
        crate::commands::ssh_copy::run(crate::cli::VmOnlyArgs { vm: Some(vm_name.clone()) })?;
    }

    println!();
    rio::ok(&format!("Setup complete: {vm_name}"));
    println!("  Guest user   : {} / {}", args.user, args.password);
    println!("  Start VM     : rusta up {vm_name}");
    println!("  Get IP       : rusta ip {vm_name}");
    println!("  SSH          : rusta ssh {vm_name}");

    Ok(0)
}

pub(crate) fn validate_name_opt(name: Option<&str>) -> Result<()> {
    let Some(name) = name else { return Ok(()) };
    let first_ok = name
        .chars()
        .next()
        .map(|c| c.is_ascii_alphanumeric())
        .unwrap_or(false);
    let rest_ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !first_ok || !rest_ok {
        return Err(Error::msg(format!(
            "invalid VM name '{name}' (must match ^[a-zA-Z0-9][a-zA-Z0-9._-]*$)"
        )));
    }
    Ok(())
}

struct ProcessGuard {
    pid: u32,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if tart::pid_alive(self.pid) {
            tart::kill_pid(self.pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_accept_alnum_dot_underscore_dash() {
        for n in ["a", "ubuntu-2404", "VM.1", "x_y", "abc123"] {
            assert!(validate_name_opt(Some(n)).is_ok(), "should accept {n}");
        }
    }

    #[test]
    fn names_reject_invalid() {
        for n in ["", "-foo", ".bar", "_baz", "has space", "x/y", "x:y", "x@y"] {
            assert!(validate_name_opt(Some(n)).is_err(), "should reject '{n}'");
        }
    }

    #[test]
    fn none_name_is_ok() {
        assert!(validate_name_opt(None).is_ok());
    }
}
