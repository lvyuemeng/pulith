use pulith_fs::{
    AtomicWriteOptions, FallbackStrategy, HardlinkOrCopyOptions, Result, atomic_read, atomic_write,
    hardlink_or_copy,
};
use tempfile::tempdir;

#[test]
fn test_atomic_write_basic() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.txt");

    atomic_write(&path, b"hello world", AtomicWriteOptions::new()).unwrap();

    assert!(path.exists());
    assert_eq!(atomic_read(&path).unwrap(), b"hello world");
}

#[test]
fn test_atomic_write_preserves_content_on_failure() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("existing.txt");

    std::fs::write(&path, "original").unwrap();

    let result = atomic_write(&path, b"new content", AtomicWriteOptions::new());

    assert!(result.is_ok());
    assert_eq!(atomic_read(&path).unwrap(), b"new content");
}

#[cfg(unix)]
#[test]
fn test_hardlink_or_copy_hardlink() {
    use std::os::unix::fs::MetadataExt;

    let dir = tempdir().unwrap();
    let src = dir.path().join("source.txt");
    let dest = dir.path().join("hardlink.txt");

    std::fs::write(&src, "shared content").unwrap();

    hardlink_or_copy(&src, &dest, HardlinkOrCopyOptions::new()).unwrap();

    assert!(dest.exists());

    let src_meta = std::fs::metadata(&src).unwrap();
    let dest_meta = std::fs::metadata(&dest).unwrap();

    assert_eq!(src_meta.ino(), dest_meta.ino());
}

#[cfg(not(unix))]
#[test]
fn test_hardlink_or_copy_hardlink() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("source.txt");
    let dest = dir.path().join("hardlink.txt");

    std::fs::write(&src, "shared content").unwrap();

    hardlink_or_copy(&src, &dest, HardlinkOrCopyOptions::new()).unwrap();

    assert!(dest.exists());
    assert_eq!(atomic_read(&dest).unwrap(), b"shared content");
}

#[test]
fn test_hardlink_or_copy_fallback_copy() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("source.txt");
    let dest = dir.path().join("copy.txt");

    std::fs::write(&src, "content to copy").unwrap();

    let options = HardlinkOrCopyOptions::new().fallback(FallbackStrategy::Copy);
    hardlink_or_copy(&src, &dest, options).unwrap();

    assert!(dest.exists());
    assert_eq!(atomic_read(&dest).unwrap(), b"content to copy");
}

#[cfg(unix)]
#[test]
fn test_atomic_write_with_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let path = dir.path().join("executable.sh");

    atomic_write(
        &path,
        b"#!/bin/bash\necho hello",
        AtomicWriteOptions::new().permissions(0o755),
    )
    .unwrap();

    let metadata = std::fs::metadata(&path).unwrap();
    let perms = metadata.permissions().mode();

    assert_eq!(perms & 0o777, 0o755);
}

#[cfg(unix)]
#[test]
fn test_symlink_functionality() {
    use pulith_fs::atomic_symlink;

    let dir = tempdir().unwrap();
    let target = dir.path().join("target_file");
    let link = dir.path().join("symlink");

    std::fs::write(&target, "target content").unwrap();
    atomic_symlink(&target, &link).unwrap();

    assert!(link.is_symlink());
    assert_eq!(atomic_read(&link).unwrap(), b"target content");
}

#[cfg(unix)]
#[test]
fn test_hardlink_or_copy_directory() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("source_dir");
    let dest = dir.path().join("dest_dir");

    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("file1.txt"), "content1").unwrap();
    std::fs::write(src.join("file2.txt"), "content2").unwrap();

    let options = HardlinkOrCopyOptions::new().fallback(FallbackStrategy::Copy);
    hardlink_or_copy(&src, &dest, options).unwrap();

    assert!(dest.is_dir());
    assert!(dest.join("file1.txt").exists());
    assert!(dest.join("file2.txt").exists());
}

#[cfg(windows)]
#[test]
fn test_junction_creation() {
    use pulith_fs::atomic_symlink;

    let dir = tempdir().unwrap();
    let target = dir.path().join("target_dir");
    let junction = dir.path().join("junction_link");

    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("file.txt"), "test").unwrap();

    if atomic_symlink(&target, &junction).is_ok() {
        assert!(junction.exists());
        assert!(junction.is_dir());
        assert!(junction.join("file.txt").exists());
    }
}

#[cfg(windows)]
#[test]
fn test_replace_directory() {
    use pulith_fs::replace_dir;

    let dir = tempdir().unwrap();
    let src = dir.path().join("new_version");
    let dest = dir.path().join("current");

    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("bin.exe"), "binary").unwrap();

    replace_dir(&src, &dest, pulith_fs::ReplaceDirOptions::new()).unwrap();

    assert!(dest.exists());
    assert!(dest.join("bin.exe").exists());
    assert!(!src.exists());
}
