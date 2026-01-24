use crate::{Error, Result};
use std::borrow::Cow;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    mmap_threshold: u64,
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_mmap_threshold(mut self, bytes: u64) -> Self {
        self.mmap_threshold = bytes;
        self
    }

    pub fn mmap_threshold(&self) -> u64 {
        self.mmap_threshold
    }
}

pub struct Resource<'a> {
    path: Cow<'a, Path>,
    options: Options,
    initial_mtime: Option<std::time::SystemTime>,
}

impl<'a> Resource<'a> {
    pub fn new(path: impl Into<Cow<'a, Path>>) -> Result<Self> {
        let path = path.into();
        let metadata = path
            .metadata()
            .map_err(|_| Error::NotFound(path.to_path_buf()))?;

        Ok(Self {
            path,
            options: Options::default(),
            initial_mtime: metadata.modified().ok(),
        })
    }

    pub fn with_options(path: impl Into<Cow<'a, Path>>, options: Options) -> Result<Self> {
        let path = path.into();
        let metadata = path
            .metadata()
            .map_err(|_| Error::NotFound(path.to_path_buf()))?;

        Ok(Self {
            path,
            options,
            initial_mtime: metadata.modified().ok(),
        })
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    pub fn ensure_integrity(&self) -> Result<()> {
        let current_mtime = self
            .path
            .metadata()
            .map_err(|e| Error::Read {
                path: self.path.to_path_buf(),
                source: e,
            })?
            .modified()
            .ok();

        if current_mtime != self.initial_mtime {
            return Err(Error::ModifiedExternally(self.path.to_path_buf()));
        }
        Ok(())
    }

    pub fn metadata(&self) -> Result<std::fs::Metadata> {
        self.path.as_ref().metadata().map_err(|e| Error::Read {
            path: self.path.to_path_buf(),
            source: e,
        })
    }

    pub fn size(&self) -> Result<u64> {
        self.metadata().map(|m| m.len())
    }

    pub fn is_dir(&self) -> bool {
        self.path.as_ref().is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.path.as_ref().is_file()
    }

    pub fn content(&self) -> Result<Content> {
        self.ensure_integrity()?;
        let size = self.size()?;

        if size < self.options.mmap_threshold() {
            let data = std::fs::read(self.path.as_ref()).map_err(|e| Error::Read {
                path: self.path.to_path_buf(),
                source: e,
            })?;
            Ok(Content::Small(data))
        } else {
            let file = std::fs::File::open(self.path.as_ref()).map_err(|e| Error::Read {
                path: self.path.to_path_buf(),
                source: e,
            })?;
            let mmap = unsafe {
                memmap2::MmapOptions::new()
                    .map(&file)
                    .map_err(|_| Error::Failed)?
            };
            Ok(Content::Mmap(mmap))
        }
    }

    pub fn read_all(self) -> Result<Vec<u8>> {
        self.content()?.to_vec()
    }
}

pub enum Content {
    Small(Vec<u8>),
    Mmap(memmap2::Mmap),
}

impl Content {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Content::Small(data) => data.as_slice(),
            Content::Mmap(mmap) => mmap.as_ref(),
        }
    }

    pub fn to_vec(self) -> Result<Vec<u8>> {
        match self {
            Content::Small(data) => Ok(data),
            Content::Mmap(mmap) => Ok(mmap.to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resource_metadata() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello").unwrap();
        let resource = Resource::new(&path).unwrap();
        assert_eq!(resource.size().unwrap(), 5);
        assert!(resource.is_file());
    }

    #[test]
    fn test_resource_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();
        let resource = Resource::new(&path).unwrap();
        let content = resource.content().unwrap();
        assert_eq!(content.as_slice(), b"hello world");
    }
}
