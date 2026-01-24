use crate::{Error, Result};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub enum FallBack {
    #[default]
    Copy,
    Error,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub fallback: FallBack,
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn fallback(mut self, fallback: FallBack) -> Self {
        self.fallback = fallback;
        self
    }
}

pub fn hardlink_or_copy(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    options: Options,
) -> Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();

    // Check if source is a directory
    let src_metadata = std::fs::metadata(src).map_err(|e| Error::Read {
        path: src.to_path_buf(),
        source: e,
    })?;

    if src_metadata.is_dir() {
        // For directories, use copy_dir_all instead of hard_link
        if matches!(options.fallback, FallBack::Copy) {
            crate::primitives::copy_dir::copy_dir_all(src, dest)
        } else {
            Err(Error::CrossDeviceHardlink)
        }
    } else {
        // For files, try hard link first
        match std::fs::hard_link(src, dest) {
            Ok(_) => Ok(()),
            Err(e)
                if e.raw_os_error() == Some(18)
                    || e.kind() == std::io::ErrorKind::CrossesDevices =>
            {
                if matches!(options.fallback, FallBack::Copy) {
                    std::fs::copy(src, dest)
                        .map(drop)
                        .map_err(|e| Error::Write {
                            path: dest.to_path_buf(),
                            source: e,
                        })
                } else {
                    Err(Error::CrossDeviceHardlink)
                }
            }
            Err(e) => Err(Error::Write {
                path: dest.to_path_buf(),
                source: e,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hardlink_or_copy() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        std::fs::write(&src, "data").unwrap();

        hardlink_or_copy(&src, &dest, Options::new()).unwrap();
        assert!(dest.exists());
    }

    #[test]
    fn test_hardlink_or_copy_cross_device() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        std::fs::write(&src, "data").unwrap();

        let options = Options::new().fallback(FallBack::Copy);
        hardlink_or_copy(&src, &dest, options).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"data");
    }
}
