use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ArchiveEntry {
    pub path: PathBuf,
    pub size: u64,
    pub permissions: Option<u32>,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_entry_fields() {
        let entry = ArchiveEntry {
            path: PathBuf::from("bin/tool"),
            size: 1024,
            permissions: Some(0o755),
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
        };
        assert_eq!(entry.path, PathBuf::from("bin/tool"));
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.permissions, Some(0o755));
        assert!(!entry.is_directory);
        assert!(!entry.is_symlink);
    }

    #[test]
    fn archive_entry_directory() {
        let entry = ArchiveEntry {
            path: PathBuf::from("bin"),
            size: 0,
            permissions: Some(0o755),
            is_directory: true,
            is_symlink: false,
            symlink_target: None,
        };
        assert!(entry.is_directory);
        assert_eq!(entry.size, 0);
    }

    #[test]
    fn archive_entry_symlink() {
        let entry = ArchiveEntry {
            path: PathBuf::from("lib/lib.so"),
            size: 0,
            permissions: Some(0o777),
            is_directory: false,
            is_symlink: true,
            symlink_target: Some(PathBuf::from("liblib.so.1")),
        };
        assert!(entry.is_symlink);
        assert_eq!(entry.symlink_target, Some(PathBuf::from("liblib.so.1")));
    }

    #[test]
    fn archive_entry_no_permissions() {
        let entry = ArchiveEntry {
            path: PathBuf::from("file.txt"),
            size: 100,
            permissions: None,
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
        };
        assert!(entry.permissions.is_none());
    }

    #[test]
    fn archive_entry_clone() {
        let entry = ArchiveEntry {
            path: PathBuf::from("src/main.rs"),
            size: 500,
            permissions: Some(0o644),
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
        };
        let cloned = entry.clone();
        assert_eq!(entry.path, cloned.path);
        assert_eq!(entry.size, cloned.size);
        assert_eq!(entry.permissions, cloned.permissions);
    }
}
