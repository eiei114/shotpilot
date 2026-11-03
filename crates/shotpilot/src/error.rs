//! User-facing errors. The CLI prints the message without a backtrace.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Bad input or a state the user can fix.
    User,
    /// Another mutation is running (the app server answers 409).
    Busy,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: Kind,
    pub msg: String,
}

impl Error {
    pub fn user(msg: impl Into<String>) -> Self {
        Error { kind: Kind::User, msg: msg.into() }
    }

    pub fn busy(msg: impl Into<String>) -> Self {
        Error { kind: Kind::Busy, msg: msg.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::user(format!("io error: {e}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::user(format!("json error: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// `bail!("...")` returns a user error from the current function.
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::error::Error::user(format!($($arg)*)))
    };
}
