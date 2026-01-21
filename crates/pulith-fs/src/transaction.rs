use std::borrow::Cow;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::{Error, Result, from_io};

pub struct Transaction<'a>(Cow<'a, Path>);

impl<'a> Transaction<'a> {
    pub fn open(path: impl Into<Cow<'a, Path>>) -> Result<Self> {
        let path = path.into();
        if !path.as_ref().exists() {
            File::create(path.as_ref()).map_err(from_io)?;
        }
        Ok(Self(path))
    }

    pub fn read(&self) -> Result<Vec<u8>> { crate::atomic_read(&self.0) }

    pub fn write(&self, content: &[u8]) -> Result<()> {
        crate::atomic_write(&self.0, content, crate::AtomicWriteOptions::new())
    }

    pub fn path(&self) -> &Path { self.0.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_transaction_new_file() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("test.bin");
        let tx = Transaction::open(path.clone())?;
        assert!(path.exists());
        Ok(())
    }

    #[test]
    fn test_transaction_read_write() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("test.bin");
        let tx = Transaction::open(&path)?;

        tx.write(b"hello")?;
        let content = tx.read()?;
        assert_eq!(content, b"hello");

        tx.write(b"world")?;
        let content = tx.read()?;
        assert_eq!(content, b"world");

        Ok(())
    }

    #[test]
    fn test_transaction_rollback() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("test.bin");
        let tx = Transaction::open(&path)?;

        tx.write(b"initial")?;

        let result = tx.write(b"fail");

        assert!(result.is_err());

        let content = tx.read()?;
        assert_eq!(content, b"initial");

        Ok(())
    }
}
