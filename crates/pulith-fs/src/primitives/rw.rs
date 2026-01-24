use crate::permissions::PermissionMode;
use crate::{Error, Result};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub permissions: Option<PermissionMode>,
    pub sync: bool,
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn permissions(mut self, mode: PermissionMode) -> Self {
        self.permissions = Some(mode);
        self
    }
    pub fn sync(mut self, sync: bool) -> Self {
        self.sync = sync;
        self
    }
}

pub fn atomic_write(path: impl AsRef<Path>, content: &[u8], options: Options) -> Result<()> {
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

    if let Some(mode) = options.permissions {
        mode.apply_to_path(&tmp_path)?;
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
        atomic_write(&path, b"hello world", Options::new()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello world");
    }

    #[test]
    fn test_atomic_write_with_custom_permissions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(
            &path,
            b"data",
            Options::new().permissions(PermissionMode::Custom(0o755)),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
        }
    }

    #[test]
    fn test_atomic_write_with_readonly() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(
            &path,
            b"data",
            Options::new().permissions(PermissionMode::ReadOnly),
        )
        .unwrap();
        let metadata = fs::metadata(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(metadata.permissions().mode() & 0o777, 0o444);
        }
        #[cfg(windows)]
        assert!(metadata.permissions().readonly());
    }

    #[test]
    fn test_atomic_write_with_inherit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        atomic_write(
            &path,
            b"data",
            Options::new().permissions(PermissionMode::Inherit),
        )
        .unwrap();
        assert!(path.exists());
    }
}
