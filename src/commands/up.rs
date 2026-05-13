use std::time::Duration;

use crate::cli::UpArgs;
use crate::error::{Error, Result};
use crate::io as rio;
use crate::picker;
use crate::tart;

pub fn run(args: UpArgs) -> Result<u8> {
    let vm = picker::resolve_vm(args.vm)?;
    if !tart::exists(&vm)? {
        return Err(Error::not_found(format!("VM '{vm}' not found")));
    }
    if tart::is_running(&vm)? {
        rio::skip(&format!("VM '{vm}' is already running"));
        return Ok(0);
    }

    let headless = !args.graphical;
    if headless {
        rio::info(&format!("Starting VM '{vm}' headlessly..."));
    } else {
        rio::info(&format!("Starting VM '{vm}' with graphics window..."));
    }
    let child = tart::run_background(&vm, headless)?;
    let _ = tart::write_pid_file(&vm, child.id());
    // Detach: leak the Child handle so the underlying process keeps running.
    std::mem::forget(child);

    rio::info("Waiting for guest agent...");
    if let Err(e) = tart::wait_for_guest_agent(&vm, Duration::from_secs(120)) {
        rio::skip(&format!("guest agent not ready: {e} (continuing)"));
    } else {
        rio::ok("Guest agent is ready");
    }

    match tart::wait_for_ip(&vm, Duration::from_secs(120)) {
        Ok(ip) => rio::ok(&format!("Guest IP: {ip}")),
        Err(e) => rio::skip(&format!("guest IP not yet available: {e}")),
    }
    Ok(0)
}
