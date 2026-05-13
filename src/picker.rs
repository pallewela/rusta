use std::io::{BufRead, IsTerminal, Write};

use crate::error::{Error, Result};
use crate::io as rio;
use crate::state;
use crate::tart::{self, Vm};

/// Resolve a VM name: explicit arg → state default → interactive picker.
/// On success, returns the VM name. Persists the picked name to state.toml.
pub fn resolve_vm(explicit: Option<String>) -> Result<String> {
    if let Some(name) = explicit {
        return Ok(name);
    }
    let st = state::State::load();
    let all = tart::list()?;
    if let Some(d) = st.default_vm.as_deref() {
        if all.iter().any(|v| v.name == d) {
            return Ok(d.to_string());
        }
        rio::skip(&format!(
            "Default VM '{d}' no longer exists; falling back to picker."
        ));
    }
    if all.is_empty() {
        return Err(Error::not_found(
            "no VMs found. Create one with `rusta create`.".to_string(),
        ));
    }
    if !std::io::stdin().is_terminal() {
        return Err(Error::not_found(
            "no default VM set and stdin is not a TTY. Pass <vm> explicitly or run `rusta default <vm>` first.".to_string(),
        ));
    }
    let picked = prompt_pick(&all, &mut std::io::stdin().lock(), &mut std::io::stdout())?;
    state::set_default(&picked)?;
    rio::ok(&format!(
        "Set '{}' as default for future commands.",
        picked
    ));
    Ok(picked)
}

/// Render the picker and read a 1-based index from `input`.
/// Exposed for unit testing — production callers go through `resolve_vm`.
pub(crate) fn prompt_pick<R: BufRead, W: Write>(
    vms: &[Vm],
    input: &mut R,
    out: &mut W,
) -> Result<String> {
    writeln!(out, "No default VM is set. Pick one:")
        .map_err(|e| Error::msg(e.to_string()))?;
    for (i, v) in vms.iter().enumerate() {
        writeln!(out, "  {}) {}   ({})", i + 1, v.name, v.status)
            .map_err(|e| Error::msg(e.to_string()))?;
    }
    write!(out, "> ").map_err(|e| Error::msg(e.to_string()))?;
    out.flush().ok();

    let mut buf = String::new();
    let n = input
        .read_line(&mut buf)
        .map_err(|e| Error::msg(e.to_string()))?;
    if n == 0 {
        return Err(Error::msg("aborted: no selection".to_string()));
    }
    let s = buf.trim();
    if s.is_empty() {
        return Err(Error::msg("aborted: no selection".to_string()));
    }
    let idx: usize = s
        .parse()
        .map_err(|_| Error::msg(format!("invalid selection '{s}'")))?;
    if idx < 1 || idx > vms.len() {
        return Err(Error::msg(format!(
            "selection {idx} out of range (1..={})",
            vms.len()
        )));
    }
    Ok(vms[idx - 1].name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vms(names: &[&str]) -> Vec<Vm> {
        names
            .iter()
            .map(|n| Vm { name: (*n).into(), status: "stopped".into() })
            .collect()
    }

    fn run(input: &str, list: &[&str]) -> Result<String> {
        let mut out = Vec::<u8>::new();
        let mut reader = std::io::Cursor::new(input.as_bytes().to_vec());
        prompt_pick(&vms(list), &mut reader, &mut out)
    }

    #[test]
    fn picks_first() {
        assert_eq!(run("1\n", &["a", "b", "c"]).unwrap(), "a");
    }

    #[test]
    fn picks_last() {
        assert_eq!(run("3\n", &["a", "b", "c"]).unwrap(), "c");
    }

    #[test]
    fn rejects_zero() {
        assert!(run("0\n", &["a"]).unwrap_err().message.contains("out of range"));
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(run("5\n", &["a", "b"]).unwrap_err().message.contains("out of range"));
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(run("hello\n", &["a"]).unwrap_err().message.contains("invalid selection"));
    }

    #[test]
    fn rejects_empty_line() {
        assert!(run("\n", &["a"]).unwrap_err().message.contains("aborted"));
    }

    #[test]
    fn rejects_eof() {
        assert!(run("", &["a"]).unwrap_err().message.contains("aborted"));
    }

    #[test]
    fn writes_menu_to_writer() {
        let mut out = Vec::<u8>::new();
        let mut input = std::io::Cursor::new(b"1\n".to_vec());
        prompt_pick(&vms(&["alpha", "beta"]), &mut input, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("alpha"));
        assert!(text.contains("beta"));
        assert!(text.contains("> "));
    }
}
