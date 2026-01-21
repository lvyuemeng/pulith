use std::borrow::Cow;
use std::path::Path;

use crate::{Result, from_io};

pub struct Workspace<'a>(Cow<'a, Path>);

impl<'a> Workspace<'a> {
    pub fn new(root: impl Into<Cow<'a, Path>>) -> Result<Self> {
        let root = root.into();
        if !root.as_ref().exists() {
            std::fs::create_dir_all(root.as_ref()).map_err(from_io)?;
        }
        Ok(Self(root))
    }

    pub fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        let full_path = self.0.as_ref().join(path);
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(from_io)?;
            }
        }
        crate::atomic_write(&full_path, content, crate::AtomicWriteOptions::new())
    }

    pub fn create_dir(&self, path: &Path) -> Result<()> {
        let full_path = self.0.as_ref().join(path);
        std::fs::create_dir(&full_path).map_err(from_io)
    }

    pub fn create_dir_all(&self, path: &Path) -> Result<()> {
        let full_path = self.0.as_ref().join(path);
        std::fs::create_dir_all(&full_path).map_err(from_io)
    }

    pub fn commit(self, destination: impl AsRef<Path>) -> Result<()> {
        let destination = destination.as_ref();
        crate::replace_dir(
            self.0.as_ref(),
            destination,
            crate::ReplaceDirOptions::new(),
        )
    }

    pub fn path(&self) -> &Path { self.0.as_ref() }
}

impl<'a> Drop for Workspace<'a> {
    fn drop(&mut self) {
        if self.0.as_ref().exists() {
            let _ = std::fs::remove_dir_all(self.0.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_workspace_write() -> Result<()> {
        let dir = tempdir()?;
        let workspace = Workspace::new(dir.path())?;
        workspace.write(Path::new("test.txt"), b"hello")?;
        assert!(dir.path().join("test.txt").exists());
        Ok(())
    }

    #[test]
    fn test_workspace_create_dir() -> Result<()> {
        let dir = tempdir()?;
        let workspace = Workspace::new(dir.path())?;
        workspace.create_dir(Path::new("subdir"))?;
        assert!(dir.path().join("subdir").is_dir());
        Ok(())
    }

    #[test]
    fn test_workspace_create_dir_all() -> Result<()> {
        let dir = tempdir()?;
        let workspace = Workspace::new(dir.path())?;
        workspace.create_dir_all(Path::new("a/b/c"))?;
        assert!(dir.path().join("a/b/c").is_dir());
        Ok(())
    }

    #[test]
    fn test_workspace_commit() -> Result<()> {
        let dir = tempdir()?;
        let workspace = Workspace::new(dir.path())?;
        workspace.write(Path::new("file.txt"), b"data")?;
        let dest = dir.path().join("dest");
        workspace.commit(&dest)?;
        assert!(dest.exists());
        Ok(())
    }

    #[test]
    fn test_workspace_cleanup_on_drop() -> Result<()> {
        let dir = tempdir()?;
        let workspace = Workspace::new(dir.path().join("staging"))?;
        workspace.write(Path::new("file.txt"), b"data")?;
        assert!(dir.path().join("staging").exists());
        drop(workspace);
        assert!(!dir.path().join("staging").exists());
        Ok(())
    }
}
