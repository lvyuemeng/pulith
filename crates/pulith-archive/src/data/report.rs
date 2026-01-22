use std::path::PathBuf;

use super::{archive::ArchiveFormat, entry::ArchiveEntry};

#[derive(Clone, Debug)]
pub struct ArchiveReport {
    pub format: ArchiveFormat,
    pub entry_count: usize,
    pub total_bytes: u64,
    pub entries: Vec<ExtractedEntry>,
}

#[derive(Clone, Debug)]
pub struct ExtractedEntry {
    pub original_path: PathBuf,
    pub target_path: PathBuf,
    pub size: u64,
    pub permissions: Option<u32>,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    pub hash: Option<String>,
}

impl ExtractedEntry {
    pub fn from_archive_entry(
        entry: ArchiveEntry,
        target_path: PathBuf,
        hash: Option<String>,
    ) -> Self {
        Self {
            original_path: entry.path,
            target_path,
            size: entry.size,
            permissions: entry.permissions,
            is_directory: entry.is_directory,
            is_symlink: entry.is_symlink,
            symlink_target: entry.symlink_target,
            hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::archive::ArchiveFormat;

    #[test]
    fn archive_report_fields() {
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 5,
            total_bytes: 1024,
            entries: Vec::new(),
        };
        assert_eq!(report.format, ArchiveFormat::Zip);
        assert_eq!(report.entry_count, 5);
        assert_eq!(report.total_bytes, 1024);
        assert!(report.entries.is_empty());
    }

    #[test]
    fn extracted_entry_fields() {
        let entry = ExtractedEntry {
            original_path: PathBuf::from("bin/tool"),
            target_path: PathBuf::from("/opt/mytool/bin/tool"),
            size: 1024,
            permissions: Some(0o755),
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
            hash: Some("abc123".to_string()),
        };
        assert_eq!(entry.original_path, PathBuf::from("bin/tool"));
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.permissions, Some(0o755));
        assert!(!entry.is_directory);
        assert!(!entry.is_symlink);
        assert_eq!(entry.hash, Some("abc123".to_string()));
    }

    #[test]
    fn extracted_entry_from_archive_entry() {
        let archive_entry = ArchiveEntry {
            path: PathBuf::from("src/main.rs"),
            size: 500,
            permissions: Some(0o644),
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
        };

        let extracted = ExtractedEntry::from_archive_entry(
            archive_entry,
            PathBuf::from("/opt/mytool/src/main.rs"),
            Some("sha256hash".to_string()),
        );

        assert_eq!(extracted.original_path, PathBuf::from("src/main.rs"));
        assert_eq!(
            extracted.target_path,
            PathBuf::from("/opt/mytool/src/main.rs")
        );
        assert_eq!(extracted.size, 500);
        assert_eq!(extracted.permissions, Some(0o644));
        assert_eq!(extracted.hash, Some("sha256hash".to_string()));
    }

    #[test]
    fn extracted_entry_symlink() {
        let entry = ExtractedEntry {
            original_path: PathBuf::from("lib/lib.so"),
            target_path: PathBuf::from("/opt/mytool/lib/lib.so"),
            size: 0,
            permissions: None,
            is_directory: false,
            is_symlink: true,
            symlink_target: Some(PathBuf::from("liblib.so.1")),
            hash: None,
        };
        assert!(entry.is_symlink);
        assert_eq!(entry.symlink_target, Some(PathBuf::from("liblib.so.1")));
    }

    #[test]
    fn extracted_entry_directory() {
        let entry = ExtractedEntry {
            original_path: PathBuf::from("bin"),
            target_path: PathBuf::from("/opt/mytool/bin"),
            size: 0,
            permissions: None,
            is_directory: true,
            is_symlink: false,
            symlink_target: None,
            hash: None,
        };
        assert!(entry.is_directory);
        assert_eq!(entry.size, 0);
    }
}
