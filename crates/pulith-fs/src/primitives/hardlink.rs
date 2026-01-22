use crate::{Error, Result};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub enum FallbackStrategy {
    #[default]
    Copy,
    Error,
}

#[derive(Clone, Copy, Debug)]
#[derive(Default)]
pub struct HardlinkOrCopyOptions {
    pub fallback: FallbackStrategy,
}


impl HardlinkOrCopyOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn fallback(mut self, fallback: FallbackStrategy) -> Self {
        self.fallback = fallback;
        self
    }
}

pub fn hardlink_or_copy(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    options: HardlinkOrCopyOptions,
) -> Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();

    match std::fs::hard_link(src, dest) {
        Ok(_) => Ok(()),
        Err(e)
            if e.raw_os_error() == Some(18) || e.kind() == std::io::ErrorKind::CrossesDevices =>
        {
            if matches!(options.fallback, FallbackStrategy::Copy) {
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

        hardlink_or_copy(&src, &dest, HardlinkOrCopyOptions::new()).unwrap();
        assert!(dest.exists());
    }

    #[test]
    fn test_hardlink_or_copy_cross_device() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        std::fs::write(&src, "data").unwrap();

        let options = HardlinkOrCopyOptions::new().fallback(FallbackStrategy::Copy);
        hardlink_or_copy(&src, &dest, options).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"data");
    }
}
