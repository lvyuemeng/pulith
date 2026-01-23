use std::path::{Path, PathBuf};

use crate::format::ArchiveFormat;

/// Represents an archive entry during extraction.
#[derive(Clone, Debug)]
pub struct Entry {
    pub original_path: PathBuf,
    pub target_path: Option<PathBuf>,
    pub size: u64,
    pub mode: Option<u32>,
    pub kind: EntryKind,
    pub hash: Option<String>,
}

impl Entry {
    pub fn new(original_path: PathBuf, size: u64, mode: Option<u32>, kind: EntryKind) -> Self {
        Self {
            original_path,
            target_path: None,
            size,
            mode,
            kind,
            hash: None,
        }
    }

    pub fn with_target_path(mut self, target_path: PathBuf) -> Self {
        self.target_path = Some(target_path);
        self
    }

    pub fn with_hash(mut self, hash: String) -> Self {
        self.hash = Some(hash);
        self
    }

    pub fn is_file(&self) -> bool {
        matches!(self.kind, EntryKind::File)
    }

    pub fn is_directory(&self) -> bool {
        matches!(self.kind, EntryKind::Directory)
    }

    pub fn is_symlink(&self) -> bool {
        matches!(self.kind, EntryKind::Symlink { .. })
    }

    pub fn symlink_target(&self) -> Option<&Path> {
        match &self.kind {
            EntryKind::Symlink { target } => Some(target),
            _ => None,
        }
    }

    /// Check if entry is executable (has execute bit set)
    pub fn is_executable(&self) -> bool {
        self.mode.map_or(false, |m| m & 0o111 != 0)
    }
}

#[derive(Clone, Debug)]
pub enum EntryKind {
    File,
    Directory,
    Symlink { target: PathBuf },
}

#[derive(Clone, Debug)]
pub struct ArchiveReport {
    pub format: ArchiveFormat,
    pub entry_count: usize,
    pub total_bytes: u64,
    pub entries: Vec<Entry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::*;

    #[test]
    fn entry_fields() {
        let entry = Entry::new(
            PathBuf::from("bin/tool"),
            1024,
            Some(0o755),
            EntryKind::File,
        );
        assert_eq!(entry.original_path, PathBuf::from("bin/tool"));
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.mode, Some(0o755));
        assert!(entry.is_file());
        assert!(!entry.is_directory());
        assert!(!entry.is_symlink());
    }

    #[test]
    fn entry_with_target_path() {
        let entry = Entry::new(
            PathBuf::from("bin/tool"),
            1024,
            Some(0o755),
            EntryKind::File,
        )
        .with_target_path(PathBuf::from("/opt/mytool/bin/tool"));
        assert_eq!(
            entry.target_path,
            Some(PathBuf::from("/opt/mytool/bin/tool"))
        );
    }

    #[test]
    fn entry_directory() {
        let entry = Entry::new(PathBuf::from("bin"), 0, Some(0o755), EntryKind::Directory);
        assert!(entry.is_directory());
    }

    #[test]
    fn entry_symlink() {
        let entry = Entry::new(
            PathBuf::from("lib/lib.so"),
            0,
            Some(0o777),
            EntryKind::Symlink {
                target: PathBuf::from("liblib.so.1"),
            },
        );
        assert!(entry.is_symlink());
        assert_eq!(entry.symlink_target(), Some(Path::new("liblib.so.1")));
    }

    #[test]
    fn entry_executable_with_mode() {
        let entry = Entry::new(
            PathBuf::from("bin/tool"),
            1024,
            Some(0o755),
            EntryKind::File,
        );
        assert!(entry.is_executable());
    }

    #[test]
    fn entry_non_executable_with_mode() {
        let entry = Entry::new(
            PathBuf::from("config/file"),
            1024,
            Some(0o644),
            EntryKind::File,
        );
        assert!(!entry.is_executable());
    }

    #[test]
    fn entry_executable_no_mode() {
        let entry = Entry::new(PathBuf::from("bin/tool"), 1024, None, EntryKind::File);
        assert!(!entry.is_executable());
    }

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
    fn archive_report_with_entries() {
        let entries = vec![
            Entry::new(
                PathBuf::from("bin/tool"),
                1024,
                Some(0o755),
                EntryKind::File,
            )
            .with_target_path(PathBuf::from("/opt/mytool/bin/tool")),
        ];
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 1,
            total_bytes: 1024,
            entries,
        };
        assert_eq!(report.entry_count, 1);
        assert_eq!(report.total_bytes, 1024);
        assert_eq!(report.entries[0].original_path, PathBuf::from("bin/tool"));
    }
}
