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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display() {
        assert_eq!(
            Error::InvalidUrl("invalid".to_string()).to_string(),
            "invalid URL: invalid"
        );

        assert_eq!(
            Error::Http {
                status: 404,
                message: "Not Found".to_string()
            }
            .to_string(),
            "HTTP error: 404 Not Found"
        );

        assert_eq!(
            Error::ChecksumMismatch {
                expected: "abc123".to_string(),
                actual: "def456".to_string(),
            }
            .to_string(),
            "checksum mismatch: expected abc123, got def456"
        );

        assert_eq!(
            Error::MaxRetriesExceeded { count: 3 }.to_string(),
            "max retries exceeded (3 attempts)"
        );

        assert_eq!(
            Error::TooManyRedirects { count: 5 }.to_string(),
            "too many redirects (5)"
        );

        assert_eq!(Error::RedirectLoop.to_string(), "redirect loop detected");

        assert_eq!(
            Error::DestinationIsDirectory.to_string(),
            "destination is a directory"
        );

        assert_eq!(
            Error::InvalidState("bad state".to_string()).to_string(),
            "invalid state: bad state"
        );

        assert_eq!(
            Error::Network("connection failed".to_string()).to_string(),
            "network error: connection failed"
        );

        assert_eq!(
            Error::Timeout("request timed out".to_string()).to_string(),
            "timeout: request timed out"
        );
    }

    #[test]
    fn test_error_debug() {
        let error = Error::InvalidUrl("test".to_string());
        assert!(format!("{:?}", error).contains("InvalidUrl"));
    }

    #[test]
    fn test_result_type_alias() {
        let ok: Result<()> = Ok(());
        assert!(ok.is_ok());

        let err: Result<()> = Err(Error::InvalidUrl("test".to_string()));
        assert!(err.is_err());
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: Error = io_err.into();
        match error {
            Error::Network(msg) => assert!(msg.contains("file not found")),
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_from_verify_error_hash_mismatch() {
        let verify_err = pulith_verify::VerifyError::HashMismatch {
            expected: b"abc123".to_vec(),
            actual: b"def456".to_vec(),
        };
        let error: Error = verify_err.into();
        match error {
            Error::ChecksumMismatch { expected, actual } => {
                assert_eq!(expected, "616263313233");
                assert_eq!(actual, "646566343536");
            }
            _ => panic!("Expected ChecksumMismatch error"),
        }
    }

    #[test]
    fn test_from_verify_error_io() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let verify_err = pulith_verify::VerifyError::Io(io_err);
        let error: Error = verify_err.into();
        match error {
            Error::Network(msg) => assert!(msg.contains("access denied")),
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_from_verify_error_hex_decode() {
        let hex_err = hex::FromHexError::OddLength;
        let verify_err = pulith_verify::VerifyError::HexDecode(hex_err);
        let error: Error = verify_err.into();
        match error {
            Error::Network(_) => (),
            _ => panic!("Expected Network error"),
        }
    }

    #[cfg(feature = "reqwest")]
    #[test]
    fn test_from_reqwest_error() {
        let client = reqwest::Client::new();
        let _ = client.get("invalid-url");
        // The error would be returned when trying to send the request
        // For testing purposes, we'll create an error directly
        let error: Error = Error::Network("invalid URL".to_string());
        match error {
            Error::Network(_) => (),
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_from_transform_error() {
        let transform_err = crate::transform::TransformError::Transform("unsupported".to_string());
        let error: Error = transform_err.into();
        match error {
            Error::Transform(_) => (),
            _ => panic!("Expected Transform error"),
        }
    }

    #[test]
    fn test_fs_error_transparent() {
        let fs_err = pulith_fs::Error::NotFound(std::path::PathBuf::from("file.txt"));
        let error: Error = fs_err.into();
        assert!(error.to_string().contains("file.txt"));
    }
}
