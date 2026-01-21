//! Error types for pulith-fetch.

use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("download failed: {0}")]
    DownloadFailed(#[source] pulith_core::fs::NetworkError),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("max retries exceeded ({count} attempts)")]
    MaxRetriesExceeded { count: u32 },

    #[error("redirect loop detected (more than 10 redirects)")]
    TooManyRedirects,

    #[error("file I/O error: {0}")]
    IoError(#[source] pulith_core::fs::FsError),

    #[error("destination path is a directory")]
    DestinationIsDirectory,

    #[error("failed to create temporary file: {0}")]
    TempFileError(#[source] io::Error),

    #[error("failed to get content length: {0}")]
    ContentLengthError(#[source] pulith_core::fs::NetworkError),

    #[error("request timeout")]
    Timeout,

    #[error("connection refused")]
    ConnectionRefused,

    #[error("network error: {0}")]
    NetworkError(#[source] pulith_core::fs::NetworkError),
}

impl From<pulith_core::fs::FsError> for FetchError {
    fn from(e: pulith_core::fs::FsError) -> Self { FetchError::IoError(e) }
}

impl From<pulith_core::fs::NetworkError> for FetchError {
    fn from(e: pulith_core::fs::NetworkError) -> Self { FetchError::DownloadFailed(e) }
}

impl From<io::Error> for FetchError {
    fn from(e: io::Error) -> Self { FetchError::IoError(pulith_core::fs::FsError::Io(e)) }
}

#[derive(Debug, Error)]
#[error("invalid SHA256 hash: {0}")]
pub struct ParseSha256HashError(pub String);
