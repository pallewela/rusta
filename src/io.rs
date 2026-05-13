use std::fs::OpenOptions;
use std::io::IsTerminal;
use std::os::fd::IntoRawFd;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(v: bool) {
    VERBOSE.store(v, Ordering::Relaxed);
}

pub fn verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

fn stdout_is_tty() -> bool {
    std::io::stdout().is_terminal()
}

fn stderr_is_tty() -> bool {
    std::io::stderr().is_terminal()
}

pub struct Color {
    pub bold: &'static str,
    pub green: &'static str,
    pub yellow: &'static str,
    pub red: &'static str,
    pub cyan: &'static str,
    pub reset: &'static str,
}

pub fn colors_for(tty: bool) -> Color {
    if tty {
        Color {
            bold: "\x1b[1m",
            green: "\x1b[0;32m",
            yellow: "\x1b[0;33m",
            red: "\x1b[0;31m",
            cyan: "\x1b[0;36m",
            reset: "\x1b[0m",
        }
    } else {
        Color { bold: "", green: "", yellow: "", red: "", cyan: "", reset: "" }
    }
}

pub fn info(msg: &str) {
    let c = colors_for(stdout_is_tty());
    println!("{}{}==> {}{}", c.bold, c.cyan, msg, c.reset);
}

pub fn ok(msg: &str) {
    let c = colors_for(stdout_is_tty());
    println!("  {}[ok]{} {}", c.green, c.reset, msg);
}

pub fn skip(msg: &str) {
    let c = colors_for(stdout_is_tty());
    println!("  {}[skip]{} {}", c.yellow, c.reset, msg);
}

pub fn err(msg: &str) {
    let c = colors_for(stderr_is_tty());
    eprintln!("  {}[error]{} {}", c.red, c.reset, msg);
}

/// Set up tee of stdout+stderr to a file via a subprocess so spawned children
/// inherit the redirected fds automatically.
pub fn setup_log_tee(path: &str) -> std::io::Result<()> {
    let parent = Path::new(path).parent();
    if let Some(p) = parent {
        if !p.as_os_str().is_empty() {
            std::fs::create_dir_all(p)?;
        }
    }
    // Pre-create the file so tee has a real target if the directory was just created.
    OpenOptions::new().create(true).append(true).open(path)?;

    let mut child = Command::new("tee")
        .arg("-a")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let tee_stdin = child.stdin.take().unwrap().into_raw_fd();
    // SAFETY: dup2 with valid fds.
    unsafe {
        if libc::dup2(tee_stdin, 1) < 0 || libc::dup2(tee_stdin, 2) < 0 {
            return Err(std::io::Error::last_os_error());
        }
        libc::close(tee_stdin);
    }
    // Don't wait for child here; it will keep running until we exit and close fds.
    std::mem::forget(child);
    println!("==> Logging all output to: {}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_off_when_not_tty() {
        let c = colors_for(false);
        assert_eq!(c.bold, "");
        assert_eq!(c.green, "");
        assert_eq!(c.reset, "");
    }

    #[test]
    fn colors_on_when_tty() {
        let c = colors_for(true);
        assert!(c.bold.contains("\x1b["));
        assert!(c.reset.contains("\x1b["));
    }

    #[test]
    fn verbose_toggle_roundtrip() {
        let prev = verbose();
        set_verbose(true);
        assert!(verbose());
        set_verbose(false);
        assert!(!verbose());
        set_verbose(prev);
    }
}
