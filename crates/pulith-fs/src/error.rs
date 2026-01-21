#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("operation failed")]
    Failed,

    #[error("path not found")]
    NotFound,

    #[error("permission denied")]
    PermissionDenied,

    #[error("already exists")]
    AlreadyExists,

    #[error("retry limit exceeded")]
    RetryLimitExceeded,

    #[error("cross-device hardlink not supported")]
    CrossDeviceHardlink,

    #[error("symlink not supported on this platform")]
    SymlinkNotSupported,

    #[error("path exceeds maximum length")]
    PathTooLong,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn from_io(err: std::io::Error) -> Error {
    match err.kind() {
        std::io::ErrorKind::NotFound => Error::NotFound,
        std::io::ErrorKind::PermissionDenied => Error::PermissionDenied,
        std::io::ErrorKind::AlreadyExists => Error::AlreadyExists,
        std::io::ErrorKind::InvalidInput => Error::Failed,
        std::io::ErrorKind::InvalidData => Error::Failed,
        _ => Error::Failed,
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self { from_io(err) }
}
