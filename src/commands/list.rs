use crate::error::Result;
use crate::state;
use crate::tart;

pub fn run() -> Result<u8> {
    let vms = tart::list()?;
    let st = state::State::load();
    let default = st.default_vm.as_deref();

    println!("{:<20} {:<10} {}", "NAME", "STATUS", "DEFAULT");
    for v in &vms {
        let mark = if default == Some(v.name.as_str()) { "*" } else { "" };
        println!("{:<20} {:<10} {}", v.name, v.status, mark);
    }
    Ok(0)
}
