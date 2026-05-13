use std::time::{Duration, Instant};

use crate::cli::DownArgs;
use crate::error::{Error, Result};
use crate::io as rio;
use crate::picker;
use crate::tart;

pub fn run(args: DownArgs) -> Result<u8> {
    let vm = picker::resolve_vm(args.vm)?;
    if !tart::exists(&vm)? {
        return Err(Error::not_found(format!("VM '{vm}' not found")));
    }
    if !tart::is_running(&vm)? {
        rio::skip(&format!("VM '{vm}' is already stopped"));
        let _ = std::fs::remove_file(crate::paths::pid_file(&vm));
        return Ok(0);
    }

    if args.force {
        rio::info(&format!("Hard-stopping VM '{vm}'..."));
        if let Some(pid) = tart::read_pid_file(&vm) {
            if tart::pid_alive(pid) {
                tart::kill_pid(pid);
            }
        }
        let _ = tart::stop(&vm); // ignore error; we'll check state below.
        // Wait briefly for state to settle.
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if !tart::is_running(&vm)? {
                tart::remove_pid_file(&vm);
                rio::ok(&format!("VM '{vm}' stopped"));
                return Ok(0);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        return Err(Error::msg(format!("VM '{vm}' still running after force stop")));
    }

    // Graceful: shutdown via guest agent, then wait.
    rio::info(&format!("Requesting graceful shutdown of '{vm}'..."));
    let _ = tart::exec_quiet(&vm, &["sudo", "shutdown", "-h", "now"]);

    let deadline = Instant::now() + Duration::from_secs(args.timeout);
    while Instant::now() < deadline {
        if !tart::is_running(&vm)? {
            tart::remove_pid_file(&vm);
            rio::ok(&format!("VM '{vm}' stopped"));
            return Ok(0);
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    Err(Error::msg(format!(
        "VM '{vm}' did not stop within {}s. Retry with `rusta down {vm} --force`.",
        args.timeout
    )))
}
