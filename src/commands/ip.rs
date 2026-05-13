use std::time::Duration;

use crate::cli::VmOnlyArgs;
use crate::error::{Error, Result};
use crate::picker;
use crate::tart;

pub fn run(args: VmOnlyArgs) -> Result<u8> {
    let vm = picker::resolve_vm(args.vm)?;
    if !tart::exists(&vm)? {
        return Err(Error::not_found(format!("VM '{vm}' not found")));
    }
    let ip = tart::wait_for_ip(&vm, Duration::from_secs(60))
        .map_err(|e| Error::msg(e.message))?;
    println!("{ip}");
    Ok(0)
}
