use crate::{Error, Result};
use fs2::FileExt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub struct Transaction {
    file: File,
    path: PathBuf,
}

impl Transaction {
    fn open_file(path: impl AsRef<Path>) -> Result<File> {
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

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = Self::open_file(path.as_ref())?;
        let path = path.as_ref().to_path_buf();

        file.lock_exclusive().map_err(|e| Error::Write {
            path: path.clone(),
            source: e,
        })?;

        Ok(Self { file, path })
    }

    pub fn try_open(path: impl AsRef<Path>) -> Result<Self> {
        let file = Self::open_file(path.as_ref())?;
        let path = path.as_ref().to_path_buf();

        file.try_lock_exclusive().map_err(|e| Error::Write {
            path: path.clone(),
            source: e,
        })?;

        Ok(Self { file, path })
    }

    pub fn open_locked(path: impl AsRef<Path>) -> Result<Self> {
        Self::open(path)
    }

    pub fn try_open_locked(path: impl AsRef<Path>) -> Result<Self> {
        Self::try_open(path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self) -> Result<Vec<u8>> {
        let mut file = self.file.try_clone().map_err(|e| Error::Read {
            path: self.path.clone(),
            source: e,
        })?;
        file.seek(SeekFrom::Start(0)).map_err(|e| Error::Read {
            path: self.path.clone(),
            source: e,
        })?;

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|e| Error::Read {
            path: self.path.clone(),
            source: e,
        })?;
        Ok(bytes)
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut file = self.file.try_clone().map_err(|e| Error::Write {
            path: self.path.clone(),
            source: e,
        })?;
        file.seek(SeekFrom::Start(0)).map_err(|e| Error::Write {
            path: self.path.clone(),
            source: e,
        })?;
        file.set_len(0).map_err(|e| Error::Write {
            path: self.path.clone(),
            source: e,
        })?;
        file.write_all(data).map_err(|e| Error::Write {
            path: self.path.clone(),
            source: e,
        })?;
        file.sync_all().map_err(|e| Error::Write {
            path: self.path.clone(),
            source: e,
        })
    }

    pub fn execute<F>(&self, operation: F) -> Result<Vec<u8>>
    where
        F: FnOnce(&[u8]) -> Result<Vec<u8>>,
    {
        let current = self.read()?;
        let next = operation(&current)?;
        self.write(&next)?;
        Ok(next)
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
        let tx = Transaction::open(&path).unwrap();
        tx.write(b"data").unwrap();
        assert_eq!(tx.read().unwrap(), b"data");
    }

    #[test]
    fn test_transaction_execute() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let tx = Transaction::open(&path).unwrap();

        let result = tx
            .execute(|bytes| {
                assert!(bytes.is_empty());
                Ok(b"version=1".to_vec())
            })
            .unwrap();

        assert_eq!(result, b"version=1");
        assert_eq!(tx.read().unwrap(), b"version=1");
        drop(tx);
        assert_eq!(std::fs::read(path).unwrap(), b"version=1");
    }

    #[test]
    fn test_transaction_try_open_locked() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin");
        let tx = Transaction::open(&path).unwrap();
        let second = Transaction::try_open(&path);

        assert!(second.is_err());
        drop(tx);
        assert!(Transaction::try_open(&path).is_ok());
    }
}
