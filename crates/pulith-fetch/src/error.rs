use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP error: {status} {message}")]
    HttpError { status: u16, message: String },

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

    #[error(transparent)]
    Fs(#[from] pulith_fs::Error),

    #[error("network error: {0}")]
    Network(String),

    #[error("timeout: {0}")]
    Timeout(String),
}

impl From<std::io::Error> for FetchError {
    fn from(e: std::io::Error) -> Self {
        FetchError::Network(e.to_string())
    }
}

impl From<pulith_verify::VerifyError> for FetchError {
    fn from(e: pulith_verify::VerifyError) -> Self {
        match e {
            pulith_verify::VerifyError::HashMismatch { expected, actual } => {
                FetchError::ChecksumMismatch {
                    expected: hex::encode(expected),
                    actual: hex::encode(actual),
                }
            }
            pulith_verify::VerifyError::Io(e) => FetchError::Network(e.to_string()),
            pulith_verify::VerifyError::HexDecode(e) => FetchError::Network(e.to_string()),
        }
    }
}
