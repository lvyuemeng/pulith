use crate::{Error, Result};
use std::path::{Path, PathBuf};

pub struct Workspace {
    staging_path: PathBuf,
    destination_path: PathBuf,
    committed: bool,
}

impl Workspace {
    pub fn new(staging_dir: impl AsRef<Path>, destination: impl AsRef<Path>) -> Result<Self> {
        let staging_path = staging_dir.as_ref().to_path_buf();
        let destination_path = destination.as_ref().to_path_buf();

        if !staging_path.exists() {
            std::fs::create_dir_all(&staging_path).map_err(|e| Error::Write {
                path: staging_path.clone(),
                source: e,
            })?;
        }

        Ok(Self {
            staging_path,
            destination_path,
            committed: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.staging_path
    }

    pub fn commit(mut self) -> Result<()> {
        crate::primitives::replace_dir(
            &self.staging_path,
            &self.destination_path,
            Default::default(),
        )?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_dir_all(&self.staging_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_workspace() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join("staging");
        let dest = dir.path().join("dest");
        let workspace = Workspace::new(&staging, &dest).unwrap();
        std::fs::write(staging.join("file.txt"), "data").unwrap();
        workspace.commit().unwrap();
        assert!(dest.join("file.txt").exists());
    }

    #[test]
    fn test_workspace_cleanup_on_drop() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join("staging");
        {
            let workspace = Workspace::new(&staging, dir.path().join("dest")).unwrap();
            std::fs::write(staging.join("file.txt"), "data").unwrap();
            assert!(staging.exists());
        }
        assert!(!staging.exists());
    }
}
