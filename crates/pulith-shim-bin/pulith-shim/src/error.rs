//! Error types for shim operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("resolution failed for command '{0}': target not found")]
    NotFound(String),

    #[error("resolution failed for command '{0}': {1}")]
    ResolveFailed(String, String),
}

pub type Result<T> = std::result::Result<T, Error>;
