use crate::{Error, Result};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct AtomicWriteOptions {
    pub permissions: Option<u32>,
    pub sync: bool,
}

impl AtomicWriteOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn permissions(mut self, mode: u32) -> Self {
        self.permissions = Some(mode);
        self
    }
    pub fn sync(mut self, sync: bool) -> Self {
        self.sync = sync;
        self
    }
}

pub fn atomic_write(
    path: impl AsRef<Path>,
    content: &[u8],
    options: AtomicWriteOptions,
) -> Result<()> {
    let path = path.as_ref();
    let parent = path.parent().ok_or_else(|| Error::Write {
        path: path.to_path_buf(),
        source: std::io::Error::other("no parent directory"),
    })?;

    let mut tmp_path = parent.to_path_buf();
    tmp_path.push(format!(".tmp.{}.pulith", uuid::Uuid::new_v4()));

    fs::write(&tmp_path, content).map_err(|e| Error::Write {
        path: tmp_path.clone(),
        source: e,
    })?;

    #[cfg(unix)]
    if let Some(mode) = options.permissions {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(mode)).map_err(|e| {
            Error::Write {
                path: tmp_path.clone(),
                source: e,
            }
        })?;
    }

    if options.sync {
        let file = fs::File::open(&tmp_path).map_err(|e| Error::Write {
            path: tmp_path.clone(),
            source: e,
        })?;
        file.sync_all().map_err(|e| Error::Write {
            path: tmp_path.clone(),
            source: e,
        })?;
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        Error::Write {
            path: path.to_path_buf(),
            source: e,
        }
    })?;

    Ok(())
}

pub fn atomic_read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();
    std::fs::read(path).map_err(|e| Error::Read {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(&path, b"hello world", AtomicWriteOptions::new()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello world");
    }

    #[test]
    fn test_atomic_write_with_permissions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(&path, b"data", AtomicWriteOptions::new().permissions(0o755)).unwrap();
        let metadata = fs::metadata(&path).unwrap();
        #[cfg(unix)]
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
    }
}
