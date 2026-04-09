use crate::primitives::{hardlink, rw};
use crate::{Error, Result};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReport {
    pub staging_root: PathBuf,
    pub destination_root: PathBuf,
    pub file_count: usize,
    pub directory_count: usize,
    pub total_bytes: u64,
}

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

    pub fn staging_path(&self) -> &Path {
        &self.staging
    }

    pub fn destination_path(&self) -> &Path {
        &self.dest
    }

    pub fn exists(&self, relative_path: impl AsRef<Path>) -> Result<bool> {
        Ok(self.resolve(relative_path)?.exists())
    }

    pub fn create_dir(&self, relative_path: impl AsRef<Path>) -> Result<()> {
        let path = self.resolve(relative_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Write {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        std::fs::create_dir(&path).map_err(|e| Error::Write { path, source: e })
    }

    pub fn create_dir_all(&self, relative_path: impl AsRef<Path>) -> Result<()> {
        let path = self.resolve(relative_path)?;
        std::fs::create_dir_all(&path).map_err(|e| Error::Write { path, source: e })
    }

    pub fn write(&self, relative_path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
        self.write_with_options(relative_path, content, rw::Options::default())
    }

    pub fn write_with_options(
        &self,
        relative_path: impl AsRef<Path>,
        content: &[u8],
        options: rw::Options,
    ) -> Result<()> {
        let path = self.resolve(relative_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Write {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        rw::atomic_write(path, content, options)
    }

    pub fn copy_file(
        &self,
        source: impl AsRef<Path>,
        relative_path: impl AsRef<Path>,
    ) -> Result<u64> {
        let source = source.as_ref();
        let path = self.resolve(relative_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Write {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        std::fs::copy(source, &path).map_err(|e| Error::Write { path, source: e })
    }

    pub fn link_or_copy_file(
        &self,
        source: impl AsRef<Path>,
        relative_path: impl AsRef<Path>,
        options: hardlink::Options,
    ) -> Result<()> {
        let source = source.as_ref();
        let path = self.resolve(relative_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Write {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        hardlink::hardlink_or_copy(source, &path, options)
    }

    pub fn read(&self, relative_path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = self.resolve(relative_path)?;
        rw::atomic_read(path)
    }

    pub fn report(&self) -> Result<WorkspaceReport> {
        let mut report = WorkspaceReport {
            staging_root: self.staging.clone(),
            destination_root: self.dest.clone(),
            file_count: 0,
            directory_count: 0,
            total_bytes: 0,
        };

        if self.staging.exists() {
            self.walk(&self.staging, &mut report)?;
        }

        Ok(report)
    }

    pub fn commit(mut self) -> Result<()> {
        crate::primitives::replace_dir::replace_dir(&self.staging, &self.dest, Default::default())?;
        self.committed = true;
        Ok(())
    }

    fn resolve(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf> {
        let relative_path = relative_path.as_ref();

        if relative_path.as_os_str().is_empty() {
            return Err(Error::InvalidInput(
                "workspace path must not be empty".to_string(),
            ));
        }

        let mut sanitized = PathBuf::new();
        for component in relative_path.components() {
            match component {
                Component::Normal(part) => sanitized.push(part),
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(Error::InvalidInput(format!(
                        "workspace path escapes staging root: {}",
                        relative_path.display()
                    )));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(Error::InvalidInput(format!(
                        "workspace path must be relative: {}",
                        relative_path.display()
                    )));
                }
            }
        }

        if sanitized.as_os_str().is_empty() {
            return Err(Error::InvalidInput(format!(
                "workspace path must contain a normal component: {}",
                relative_path.display()
            )));
        }

        Ok(self.staging.join(sanitized))
    }

    fn walk(&self, path: &Path, report: &mut WorkspaceReport) -> Result<()> {
        for entry in std::fs::read_dir(path).map_err(|e| Error::Read {
            path: path.to_path_buf(),
            source: e,
        })? {
            let entry = entry.map_err(|e| Error::Read {
                path: path.to_path_buf(),
                source: e,
            })?;
            let entry_path = entry.path();
            let metadata = entry.metadata().map_err(|e| Error::Read {
                path: entry_path.clone(),
                source: e,
            })?;

            if metadata.is_dir() {
                report.directory_count += 1;
                self.walk(&entry_path, report)?;
            } else {
                report.file_count += 1;
                report.total_bytes += metadata.len();
            }
        }

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
        workspace.write("file.txt", b"data").unwrap();
        workspace.commit().unwrap();
        assert!(dest.join("file.txt").exists());
    }

    #[test]
    fn test_workspace_cleanup_on_drop() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join("staging");
        {
            let workspace = Workspace::new(&staging, dir.path().join("dest")).unwrap();
            workspace.write("file.txt", b"data").unwrap();
            assert!(staging.exists());
        }
        assert!(!staging.exists());
    }

    #[test]
    fn test_workspace_create_dirs_and_report() {
        let dir = tempdir().unwrap();
        let workspace =
            Workspace::new(dir.path().join("staging"), dir.path().join("dest")).unwrap();

        workspace.create_dir("bin").unwrap();
        workspace.create_dir_all("lib/nested").unwrap();
        workspace.write("bin/tool", b"hello").unwrap();
        workspace.write("lib/nested/config.toml", b"abc").unwrap();

        let report = workspace.report().unwrap();
        assert_eq!(report.file_count, 2);
        assert_eq!(report.directory_count, 3);
        assert_eq!(report.total_bytes, 8);
    }

    #[test]
    fn test_workspace_rejects_escaping_path() {
        let dir = tempdir().unwrap();
        let workspace =
            Workspace::new(dir.path().join("staging"), dir.path().join("dest")).unwrap();

        let result = workspace.write("../escape.txt", b"data");
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }

    #[test]
    fn test_workspace_link_or_copy_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        std::fs::write(&source, b"data").unwrap();

        let workspace =
            Workspace::new(dir.path().join("staging"), dir.path().join("dest")).unwrap();
        workspace
            .link_or_copy_file(&source, "bin/tool.txt", hardlink::Options::new())
            .unwrap();

        assert_eq!(workspace.read("bin/tool.txt").unwrap(), b"data");
    }
}
