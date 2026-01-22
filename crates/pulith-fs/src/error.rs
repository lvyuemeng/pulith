use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("path not found: {}", .0.display())]
    NotFound(PathBuf),

    #[error("resource modified externally: {}", .0.display())]
    ModifiedExternally(PathBuf),

    #[error("atomic write failed at {}: {}", .path.display(), .source)]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("atomic read failed at {}: {}", .path.display(), .source)]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("directory replace failed at {}: {}", .path.display(), .source)]
    ReplaceDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("retry limit exceeded after {0} attempts")]
    RetryLimitExceeded(u32),

    #[error("cross-device hardlink not supported")]
    CrossDeviceHardlink,

    #[error("operation failed")]
    Failed,
}
