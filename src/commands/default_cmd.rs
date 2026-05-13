use crate::cli::DefaultArgs;
use crate::error::{Error, Result};
use crate::state;
use crate::tart;

pub fn run(args: DefaultArgs) -> Result<u8> {
    match args.vm {
        None => {
            let st = state::State::load();
            match st.default_vm {
                Some(v) => {
                    println!("{v}");
                    Ok(0)
                }
                None => {
                    println!("no default set");
                    Ok(1)
                }
            }
        }
        Some(name) => {
            if !tart::exists(&name)? {
                return Err(Error::not_found(format!("VM '{name}' not found")));
            }
            state::set_default(&name)?;
            Ok(0)
        }
    }
}
