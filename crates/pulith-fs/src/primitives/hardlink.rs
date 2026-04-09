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

    if src.is_dir() {
        if matches!(options.fallback, FallBack::Copy) {
            return crate::primitives::copy_dir::copy_dir_all(src, dest);
        }

        return Err(Error::Write {
            path: dest.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "hard-linking directories is not supported",
            ),
        });
    }

    match std::fs::hard_link(src, dest) {
        Ok(_) => Ok(()),
        Err(e)
            if e.raw_os_error() == Some(18) || e.kind() == std::io::ErrorKind::CrossesDevices =>
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

    #[test]
    fn test_hardlink_or_copy_directory_with_copy_fallback() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dest = dir.path().join("dest_dir");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("file.txt"), "data").unwrap();

        let options = Options::new().fallback(FallBack::Copy);
        hardlink_or_copy(&src, &dest, options).unwrap();

        assert!(dest.is_dir());
        assert_eq!(std::fs::read(dest.join("file.txt")).unwrap(), b"data");
    }
}
