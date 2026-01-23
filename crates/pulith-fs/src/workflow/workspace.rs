use crate::{Error, Result};
use std::path::{Path, PathBuf};

pub struct Workspace {
    staging: PathBuf,
    dest: PathBuf,
    committed: bool,
}

impl Workspace {
    pub fn new(staging_dir: impl AsRef<Path>, dest_dir: impl AsRef<Path>) -> Result<Self> {
        let staging_path = staging_dir.as_ref().to_path_buf();
        let destination_path = dest_dir.as_ref().to_path_buf();

        if !staging_path.exists() {
            std::fs::create_dir_all(&staging_path).map_err(|e| Error::Write {
                path: staging_path.clone(),
                source: e,
            })?;
        }

        Ok(Self {
            staging: staging_path,
            dest: destination_path,
            committed: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.staging
    }

    pub fn commit(mut self) -> Result<()> {
        crate::primitives::replace_dir::replace_dir(&self.staging, &self.dest, Default::default())?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_dir_all(&self.staging);
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
