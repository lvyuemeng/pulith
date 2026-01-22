use std::io;

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("checksum mismatch: expected {expected:?}, got {actual:?}")]
    Mismatch {
        expected: Vec<u8>,
        actual:   Vec<u8>,
    },

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("illegal state: {0}")]
    IllegalState(&'static str),
}

pub type Result<T> = std::result::Result<T, VerificationError>;
