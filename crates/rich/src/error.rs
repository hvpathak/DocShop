use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors are written for the person using the app; the UI shows them as-is.
#[derive(Debug)]
pub enum Error {
    /// A file that is damaged or not in the expected format.
    Format(String),
    /// The file is larger than HarshPad will open.
    TooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Format(msg) => write!(f, "{msg}"),
            Error::TooLarge => write!(f, "the document is too large to open"),
        }
    }
}

impl std::error::Error for Error {}

pub(crate) fn damaged(e: impl fmt::Display) -> Error {
    Error::Format(format!("the document is damaged or not supported: {e}"))
}
