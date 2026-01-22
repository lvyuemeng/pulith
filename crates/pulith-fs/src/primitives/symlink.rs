use crate::{Error, Result};
use std::path::Path;

pub fn atomic_symlink(target: impl AsRef<Path>, link: impl AsRef<Path>) -> Result<()> {
    let target = target.as_ref();
    let link = link.as_ref();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| Error::Write {
            path: link.to_path_buf(),
            source: e,
        })
    }

    #[cfg(windows)]
    {
        if target.is_dir() {
            junction::create(target, link).map_err(|e| Error::Write {
                path: link.to_path_buf(),
                source: e,
            })
        } else {
            std::os::windows::fs::symlink_file(target, link).map_err(|e| Error::Write {
                path: link.to_path_buf(),
                source: e,
            })
        }
    }
}
