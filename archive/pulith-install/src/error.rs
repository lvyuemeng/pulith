//! Error types for install operations.

use std::path::PathBuf;
use thiserror::Error;

/// Top-level install error.
pub type InstallError = PipelineError;

/// Pipeline execution errors.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("stage failed: {0}")]
    Stage(#[from] StageError),
    #[error("transform failed: {0}")]
    Transform(Box<dyn std::error::Error + Send + Sync>),
    #[error("activate failed: {0}")]
    Activate(#[from] ActivateError),
    #[error("hook failed: {0}")]
    Hook(#[from] HookError),
    #[error("rollback failed: {0}")]
    Rollback(std::io::Error),
}

/// Staging phase errors.
#[derive(Debug, Error)]
pub enum StageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("source not found: {0}")]
    SourceNotFound(PathBuf),
}

/// Activation phase errors.
#[derive(Debug, Error)]
pub enum ActivateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("link already exists: {0}")]
    LinkExists(PathBuf),
}

/// Hook execution errors.
#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook failed: {name}: {source}")]
    HookFailed {
        name:   String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
