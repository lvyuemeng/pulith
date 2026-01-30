use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP error: {status} {message}")]
    Http { status: u16, message: String },

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("max retries exceeded ({count} attempts)")]
    MaxRetriesExceeded { count: u32 },

    #[error("too many redirects ({count})")]
    TooManyRedirects { count: u32 },

    #[error("redirect loop detected")]
    RedirectLoop,

    #[error("destination is a directory")]
    DestinationIsDirectory,

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error(transparent)]
    Fs(#[from] pulith_fs::Error),

    #[error("network error: {0}")]
    Network(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("transform error: {0}")]
    Transform(#[from] crate::transform::TransformError),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Network(e.to_string())
    }
}

impl From<pulith_verify::VerifyError> for Error {
    fn from(e: pulith_verify::VerifyError) -> Self {
        match e {
            pulith_verify::VerifyError::HashMismatch { expected, actual } => {
                Error::ChecksumMismatch {
                    expected: hex::encode(expected),
                    actual: hex::encode(actual),
                }
            }
            pulith_verify::VerifyError::Io(e) => Error::Network(e.to_string()),
            pulith_verify::VerifyError::HexDecode(e) => Error::Network(e.to_string()),
        }
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Network(e.to_string())
    }
}
