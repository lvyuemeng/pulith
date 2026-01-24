use std::io;
use thiserror::Error;

/// Error types for verification operations.
///
/// Follows the error handling patterns specified in [AGENT.md](../../docs/AGENT.md).
#[derive(Error, Debug)]
pub enum VerifyError {
    /// Hash mismatch between expected and actual digest
    #[error("hash mismatch: expected {expected:?}, got {actual:?}")]
    HashMismatch {
        /// The expected hash digest
        expected: Vec<u8>,
        /// The actual computed hash digest
        actual: Vec<u8>,
    },

    /// I/O error during verification process
    #[error("I/O error during verification: {0}")]
    Io(#[from] io::Error),

    /// Hexadecimal decoding error when parsing expected hash
    #[error("hex decoding error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}

/// Result type alias for verification operations.
pub type Result<T> = std::result::Result<T, VerifyError>;
