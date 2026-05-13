use crate::cli::DeleteArgs;
use crate::commands::confirm;
use crate::error::{Error, Result};
use crate::io as rio;
use crate::state;
use crate::tart;

pub fn run(args: DeleteArgs) -> Result<u8> {
    let vm = args.vm;
    if !tart::exists(&vm)? {
        return Err(Error::not_found(format!("VM '{vm}' not found")));
    }
    if tart::is_running(&vm)? {
        if !args.force_running {
            return Err(Error::msg(format!(
                "VM '{vm}' is currently running. Run `rusta down {vm}` first, or pass --force-running."
            )));
        }
        rio::info(&format!("Stopping running VM '{vm}' before deletion..."));
        let _ = tart::stop(&vm);
    }

    if !args.yes && !confirm(&format!("Delete VM '{vm}'?"))? {
        rio::skip("aborted");
        return Ok(0);
    }

    tart::delete(&vm)?;
    state::clear_default_if_matches(&vm)?;
    let _ = state::forget_vm(&vm);
    let _ = std::fs::remove_file(crate::paths::pid_file(&vm));
    rio::ok(&format!("VM '{vm}' deleted"));
    Ok(0)
}
