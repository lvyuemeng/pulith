use crate::{Error, Result};
use fs2::FileExt;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct Transaction {
    file: File,
    path: PathBuf,
}

impl Transaction {
    fn open(path: impl AsRef<Path>) -> Result<File> {
        File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| Error::Write {
                path: path.as_ref().to_path_buf(),
                source: e,
            })
    }

    pub fn open_locked(path: impl AsRef<Path>) -> Result<Self> {
        let file = Self::open(path.as_ref())?;

        let path = path.as_ref().to_path_buf();
        file.lock_exclusive().map_err(|e| Error::Write {
            path: path.clone(),
            source: e,
        })?;

        Ok(Self { file, path })
    }

    pub fn try_open_locked(path: impl AsRef<Path>) -> Result<Self> {
        let file = Self::open(path.as_ref())?;

        let path = path.as_ref().to_path_buf();
        file.try_lock_exclusive().map_err(|e| Error::Write {
            path: path.clone(),
            source: e,
        })?;

        Ok(Self { file, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self) -> Result<Vec<u8>> {
        std::fs::read(&self.path).map_err(|e| Error::Read {
            path: self.path.clone(),
            source: e,
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        crate::primitives::rw::atomic_write(&self.path, data, Default::default())
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_transaction_lock() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin");
        let tx = Transaction::open_locked(&path).unwrap();
        tx.write(b"data").unwrap();
        assert_eq!(tx.read().unwrap(), b"data");
    }
}
