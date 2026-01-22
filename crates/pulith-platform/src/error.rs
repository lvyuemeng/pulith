use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown architecture: {0}")]
    UnknownArch(String),

    #[error("unknown operating system: {0}")]
    UnknownOS(String),

    #[error("unknown distribution: {0}")]
    UnknownDistro(String),

    #[error("unknown target triple: {0}")]
    UnknownTriple(String),

    #[error("unknown shell: {0}")]
    UnknownShell(String),

    #[error("failed to read distro info: {0}")]
    DistroRead(#[source] std::io::Error),

    #[error("command not found: {cmd}")]
    CommandNotFound { cmd: String },

    #[error("command failed: {cmd}, source: {source}")]
    CommandFailed { cmd: String, source: std::io::Error },

    #[error("operation failed")]
    Failed,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
