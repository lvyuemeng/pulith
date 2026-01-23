use crate::{Error, Result};
use std::fs;
use std::path::Path;

pub fn copy_dir_all(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();

    if !dest.exists() {
        fs::create_dir_all(dest).map_err(|e| Error::Write {
            path: dest.to_path_buf(),
            source: e,
        })?;
    }

    for entry in fs::read_dir(src).map_err(|e| Error::Read {
        path: src.to_path_buf(),
        source: e,
    })? {
        let entry = entry.map_err(|e| Error::Read {
            path: src.to_path_buf(),
            source: e,
        })?;
        let file_type = entry.file_type().map_err(|e| Error::Read {
            path: entry.path(),
            source: e,
        })?;

        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dest_path)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(&src_path).map_err(|e| Error::Read {
                path: src_path,
                source: e,
            })?;
            crate::primitives::symlink::atomic_symlink(target, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path).map_err(|e| Error::Write {
                path: dest_path,
                source: e,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_copy_dir_all() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&src.join("subdir")).unwrap();
        std::fs::write(src.join("file.txt"), "data").unwrap();
        std::fs::write(src.join("subdir/nested.txt"), "nested").unwrap();

        copy_dir_all(&src, &dest).unwrap();
        assert!(dest.join("file.txt").exists());
        assert!(dest.join("subdir/nested.txt").exists());
    }
}
