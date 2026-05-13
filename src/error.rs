use std::fmt;

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub exit_code: u8,
}

impl Error {
    pub fn msg<S: Into<String>>(m: S) -> Self {
        Self { message: m.into(), exit_code: 1 }
    }
    pub fn with_code<S: Into<String>>(m: S, code: u8) -> Self {
        Self { message: m.into(), exit_code: code }
    }
    pub fn not_found<S: Into<String>>(m: S) -> Self {
        Self::with_code(m, 2)
    }
    pub fn cmd(what: &str, e: std::io::Error) -> Self {
        Self::msg(format!("failed to run {}: {}", what, e))
    }
    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::msg(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_exit_code_one() {
        let e = Error::msg("x");
        assert_eq!(e.exit_code(), 1);
        assert_eq!(format!("{e}"), "x");
    }

    #[test]
    fn not_found_uses_code_two() {
        assert_eq!(Error::not_found("nope").exit_code(), 2);
    }

    #[test]
    fn with_code_uses_provided() {
        assert_eq!(Error::with_code("x", 7).exit_code(), 7);
    }

    #[test]
    fn cmd_wraps_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "boom");
        let e = Error::cmd("widget", io_err);
        assert!(e.message.contains("widget"));
        assert!(e.message.contains("boom"));
    }

    #[test]
    fn from_io_error_default_exit_one() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "bad");
        let e: Error = io_err.into();
        assert_eq!(e.exit_code(), 1);
    }
}
