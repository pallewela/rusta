use crate::cli::{GuiMode, SetGuiArgs};
use crate::error::{Error, Result};
use crate::io as rio;
use crate::state;
use crate::tart;

pub fn run(args: SetGuiArgs) -> Result<u8> {
    if !tart::exists(&args.vm)? {
        return Err(Error::not_found(format!("VM '{}' not found", args.vm)));
    }
    let gui = matches!(args.mode, GuiMode::On);
    state::set_vm_gui(&args.vm, gui).map_err(|e| Error::msg(e.to_string()))?;
    let summary = if gui {
        format!("'{}' will boot with a graphics window by default", args.vm)
    } else {
        format!("'{}' will boot headlessly by default", args.vm)
    };
    rio::ok(&summary);
    rio::info("Override per-invocation with --gui or --no-gui on `rusta up`.");
    Ok(0)
}
