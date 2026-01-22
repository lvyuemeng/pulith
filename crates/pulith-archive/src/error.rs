use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported archive format")]
    UnsupportedFormat,

    #[error("zip-slip attack detected: entry '{entry}' resolves to '{resolved}'")]
    ZipSlip { entry: PathBuf, resolved: PathBuf },

    #[error("symlink target escapes base directory: '{target}' -> '{resolved}'")]
    SymlinkEscape { target: PathBuf, resolved: PathBuf },

    #[error("symlink target is absolute path: '{target}' in '{symlink}'")]
    AbsoluteSymlinkTarget { target: PathBuf, symlink: PathBuf },

    #[error("entry path contains null byte")]
    InvalidPath,

    #[error("strip_components({count}) removed all path components from '{original}'")]
    NoComponentsRemaining { original: PathBuf, count: usize },

    #[error("failed to extract '{path}': {source}")]
    ExtractionFailed { path: PathBuf, source: io::Error },

    #[error("workspace operation failed: {source}")]
    WorkspaceFailed { source: pulith_fs::Error },

    #[error("archive is corrupted")]
    Corrupted,

    #[error("failed to create symlink: {source}")]
    SymlinkCreationFailed {
        target: PathBuf,
        link: PathBuf,
        source: io::Error,
    },

    #[error("failed to create directory: {path}: {source}")]
    DirectoryCreationFailed { path: PathBuf, source: io::Error },

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<pulith_fs::Error> for Error {
    fn from(e: pulith_fs::Error) -> Self {
        Self::WorkspaceFailed { source: e }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
