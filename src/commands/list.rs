use crate::error::Result;
use crate::state;
use crate::tart;

pub fn run() -> Result<u8> {
    let vms = tart::list()?;
    let st = state::State::load();
    let default = st.default_vm.as_deref();

    let name_w = vms.iter().map(|v| v.name.len()).max().unwrap_or(0).max("NAME".len());
    let status_w = vms.iter().map(|v| v.status.len()).max().unwrap_or(0).max("STATUS".len());

    println!("{:<name_w$}  {:<status_w$}  {}", "NAME", "STATUS", "DEFAULT");
    for v in &vms {
        let mark = if default == Some(v.name.as_str()) { "*" } else { "" };
        println!("{:<name_w$}  {:<status_w$}  {}", v.name, v.status, mark);
    }
    Ok(0)
}
