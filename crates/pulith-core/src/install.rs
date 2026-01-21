//! Atomic file operations for staged installs.
//!
//! Provides `atomic_replace` for safe file/directory replacement
//! with same-FS optimization and cross-FS fallback.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AtomicReplaceError {
    #[error("source path does not exist: {0}")]
    SourceNotFound(PathBuf),

    #[error("destination parent directory does not exist: {0}")]
    DestinationParentNotFound(PathBuf),

    #[error("failed to create parent directory: {0}")]
    CreateParentFailed(#[source] io::Error),

    #[error("failed to rename (same-FS): {0}")]
    RenameFailed(#[source] io::Error),

    #[error("failed to copy to staging location: {0}")]
    CopyFailed(#[source] io::Error),

    #[error("failed to remove stale destination: {0}")]
    RemoveStaleFailed(#[source] io::Error),

    #[error("failed to rename from staging: {0}")]
    FinalRenameFailed(#[source] io::Error),

    #[error("failed to clean up staging after failure: {0}")]
    CleanupFailed(PathBuf, #[source] io::Error),
}

fn is_same_filesystem(_p1: &Path, _p2: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(m1), Ok(m2)) = (fs::metadata(p1), fs::metadata(p2)) {
            return m1.dev() == m2.dev();
        }
    }
    false
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), io::Error> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn remove_all(path: &Path) -> Result<(), io::Error> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                remove_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        fs::remove_dir(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn atomic_replace(src: &Path, dst: &Path) -> Result<(), AtomicReplaceError> {
    if !src.exists() {
        return Err(AtomicReplaceError::SourceNotFound(src.to_path_buf()));
    }

    let dst_parent = dst
        .parent()
        .ok_or_else(|| AtomicReplaceError::DestinationParentNotFound(dst.to_path_buf()))?;

    if !dst_parent.exists() {
        fs::create_dir_all(dst_parent).map_err(AtomicReplaceError::CreateParentFailed)?;
    }

    if is_same_filesystem(src, dst) {
        match fs::rename(src, dst) {
            Ok(()) => return Ok(()),
            Err(e) => return Err(AtomicReplaceError::RenameFailed(e)),
        }
    }

    let staging = dst_parent.join(format!(
        ".pulith_staging_{}",
        dst.file_name().and_then(|n| n.to_str()).unwrap_or("tmp")
    ));

    if staging.exists() {
        let _ = remove_all(&staging);
    }

    if src.is_dir() {
        copy_dir_all(src, &staging).map_err(AtomicReplaceError::CopyFailed)?;
    } else {
        fs::copy(src, &staging).map_err(AtomicReplaceError::CopyFailed)?;
    }

    if dst.exists() {
        remove_all(dst).map_err(AtomicReplaceError::RemoveStaleFailed)?;
    }

    match fs::rename(&staging, dst) {
        Ok(()) => {
            let _ = remove_all(src);
            Ok(())
        }
        Err(e) => {
            let cleanup_err = remove_all(&staging)
                .map_err(|se| AtomicReplaceError::CleanupFailed(staging.clone(), se));
            cleanup_err?;
            Err(AtomicReplaceError::FinalRenameFailed(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dirs() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("source");
        let dst = temp.path().join("destination");
        (temp, src, dst)
    }

    fn create_file(path: &Path, content: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn assert_file_content(path: &Path, expected: &[u8]) {
        let content = fs::read(path).unwrap();
        assert_eq!(content, expected);
    }

    #[test]
    fn test_atomic_replace_file_same_fs() {
        let (_temp, src, dst) = setup_test_dirs();
        create_file(&src, b"hello world");
        atomic_replace(&src, &dst).unwrap();
        assert!(!src.exists());
        assert_file_content(&dst, b"hello world");
    }

    #[test]
    fn test_atomic_replace_file_content_preserved() {
        let (_temp, src, dst) = setup_test_dirs();
        let content = b"test content 12345";
        create_file(&src, content);
        atomic_replace(&src, &dst).unwrap();
        assert_file_content(&dst, content);
    }

    #[test]
    fn test_atomic_replace_directory() {
        let (_temp, src, dst) = setup_test_dirs();
        create_file(&src.join("file1.txt"), b"content1");
        create_file(&src.join("subdir").join("file2.txt"), b"content2");
        atomic_replace(&src, &dst).unwrap();
        assert!(!src.exists());
        assert!(dst.is_dir());
        assert_file_content(&dst.join("file1.txt"), b"content1");
        assert_file_content(&dst.join("subdir").join("file2.txt"), b"content2");
    }

    #[test]
    fn test_atomic_replace_overwrites_existing() {
        let (_temp, src, dst) = setup_test_dirs();
        create_file(&dst, b"old content");
        create_file(&src, b"new content");
        atomic_replace(&src, &dst).unwrap();
        assert_file_content(&dst, b"new content");
    }

    #[test]
    fn test_atomic_replace_creates_parent() {
        let (_temp, src, dst) = setup_test_dirs();
        let nested = dst.join("nested").join("dir");
        create_file(&src, b"content");
        atomic_replace(&src, &nested).unwrap();
        assert_file_content(&nested, b"content");
    }

    #[test]
    fn test_atomic_replace_source_not_found() {
        let (_temp, src, dst) = setup_test_dirs();
        let result = atomic_replace(&src, &dst);
        assert!(matches!(result, Err(AtomicReplaceError::SourceNotFound(_))));
    }

    #[test]
    fn test_atomic_replace_empty_dir() {
        let (_temp, src, dst) = setup_test_dirs();
        fs::create_dir_all(&src).unwrap();
        atomic_replace(&src, &dst).unwrap();
        assert!(!src.exists());
        assert!(dst.is_dir());
    }
}
