//! Store layout for managed package installations.
//!
//! Defines the directory structure for storing installed packages,
//! versions, and staging areas.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreLayoutError {
    #[error("failed to create directory: {0}")]
    CreateDir(#[source] std::io::Error),

    #[error("failed to create symlink: {0}")]
    CreateSymlink(#[source] std::io::Error),

    #[error("store root is not a directory")]
    RootNotDirectory,

    #[error("version path is not a directory")]
    VersionNotDirectory,
}

#[derive(Debug, Clone)]
pub struct StoreLayout {
    root:     PathBuf,
    versions: PathBuf,
    current:  PathBuf,
    staging:  PathBuf,
}

impl StoreLayout {
    pub fn builder() -> StoreLayoutBuilder { StoreLayoutBuilder::new() }

    pub fn root(&self) -> &Path { &self.root }

    pub fn versions(&self) -> &Path { &self.versions }

    pub fn current(&self) -> &Path { &self.current }

    pub fn staging(&self) -> &Path { &self.staging }

    pub fn version(&self, name: &str) -> PathBuf { self.versions.join(name) }

    pub fn version_bin(&self, name: &str, bin: &str) -> PathBuf {
        self.version(name).join("bin").join(bin)
    }
}

#[derive(Debug, Default)]
pub struct StoreLayoutBuilder {
    root: Option<PathBuf>,
}

impl StoreLayoutBuilder {
    pub fn new() -> Self { Self { root: None } }

    pub fn root(mut self, path: impl Into<PathBuf>) -> Self {
        self.root = Some(path.into());
        self
    }

    pub fn build(self) -> Result<StoreLayout, std::io::Error> {
        let root = self.root.unwrap_or_else(|| PathBuf::from(".pulith"));

        Ok(StoreLayout {
            root:     root.clone(),
            versions: root.join("versions"),
            current:  root.join("current"),
            staging:  root.join("staging"),
        })
    }
}

pub fn ensure_layout(layout: &StoreLayout) -> Result<(), StoreLayoutError> {
    std::fs::create_dir_all(&layout.versions).map_err(StoreLayoutError::CreateDir)?;

    std::fs::create_dir_all(&layout.staging).map_err(StoreLayoutError::CreateDir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_layout() -> (tempfile::TempDir, StoreLayout) {
        let temp = tempfile::tempdir().unwrap();
        let layout = StoreLayout::builder()
            .root(temp.path().join("store"))
            .build()
            .unwrap();
        (temp, layout)
    }

    #[test]
    fn test_store_layout_builder_default() {
        let layout = StoreLayout::builder().build().unwrap();
        assert_eq!(layout.root(), Path::new(".pulith"));
        assert_eq!(layout.versions(), Path::new(".pulith/versions"));
        assert_eq!(layout.current(), Path::new(".pulith/current"));
        assert_eq!(layout.staging(), Path::new(".pulith/staging"));
    }

    #[test]
    fn test_store_layout_builder_custom_root() {
        let layout = StoreLayout::builder().root("/custom/root").build().unwrap();
        assert_eq!(layout.root(), Path::new("/custom/root"));
        assert_eq!(layout.versions(), Path::new("/custom/root/versions"));
        assert_eq!(layout.current(), Path::new("/custom/root/current"));
    }

    #[test]
    fn test_ensure_layout_creates_directories() {
        let (_temp, layout) = temp_layout();
        ensure_layout(&layout).unwrap();

        assert!(layout.versions().exists());
        assert!(layout.staging().exists());
    }

    #[test]
    fn test_ensure_layout_idempotent() {
        let (_temp, layout) = temp_layout();
        ensure_layout(&layout).unwrap();
        ensure_layout(&layout).unwrap();

        assert!(layout.versions().exists());
        assert!(layout.staging().exists());
    }

    #[test]
    fn test_version_path() {
        let layout = StoreLayout::builder().root("/test/root").build().unwrap();
        assert_eq!(
            layout.version("1.0.0"),
            PathBuf::from("/test/root/versions/1.0.0")
        );
        assert_eq!(
            layout.version("v18"),
            PathBuf::from("/test/root/versions/v18")
        );
    }

    #[test]
    fn test_version_bin_path() {
        let layout = StoreLayout::builder().root("/test/root").build().unwrap();
        assert_eq!(
            layout.version_bin("1.0.0", "node"),
            PathBuf::from("/test/root/versions/1.0.0/bin/node")
        );
        assert_eq!(
            layout.version_bin("18.0.0", "npm"),
            PathBuf::from("/test/root/versions/18.0.0/bin/npm")
        );
    }
}
