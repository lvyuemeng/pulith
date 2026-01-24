//! Archive extraction support for ZIP and TAR formats.
//!
//! # Platform Behavior
//!
//! **Unix**: Full permission support with all `PermissionStrategy` variants functional.
//! File mode bits from archives are respected and applied according to the selected strategy.
//!
//! **Windows (non-Unix)**: Permission handling is a no-op - `PermissionStrategy` has no effect,
//! and permission values are ignored. The API accepts permission-related options for API compatibility,
//! but they are not applied on Windows platforms.

use core::marker::PhantomData;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::format;
use crate::options::{self, ExtractOptions};
use pulith_fs::workflow::Workspace;

mod tar;
mod zip;

pub use tar::TarArchive;
pub use zip::ZipSource;

pub trait EntrySource {
    /// The specific Reader type yielded by this source (e.g., ZipFile<'a> or tar::Entry<'a>).
    /// It must implement Read and be bound by the lifetime of the source ('a).
    type Reader<'a>: Read + 'a
    where
        Self: 'a;

    /// Advances to the next entry.
    /// The returned Entry borrows from `self`, preventing concurrent access to the archive.
    fn next_entry(&mut self) -> Option<Result<PendingEntry<'_, Self::Reader<'_>>>>;

    fn format(&self) -> format::ArchiveFormat;
}

/// Represents an archive entry during extraction.
#[derive(Clone, Debug)]
pub struct Entry {
    pub original_path: PathBuf,
    pub target_path: PathBuf,
    pub size: u64,
    pub mode: Option<u32>,
    pub kind: EntryKind,
    pub hash: Option<String>,
}

impl Entry {
    pub fn transit<R: Read>(
        pending: PendingEntry<R>,
        target_path: PathBuf,
        hash: Option<String>,
    ) -> Self {
        Self {
            original_path: pending.original_path,
            target_path,
            size: pending.size,
            mode: pending.mode,
            kind: EntryKind::from_pending(&pending.kind),
            hash,
        }
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
        self.mode.is_some_and(|m| m & 0o111 != 0)
    }
}

#[derive(Debug, Clone)]
pub enum EntryKind {
    File,
    Directory,
    Symlink { target: PathBuf },
}

impl EntryKind {
    fn from_pending(kind: &PendingEntryKind<impl Read>) -> Self {
        match kind {
            PendingEntryKind::File(_) => Self::File,
            PendingEntryKind::Directory => Self::Directory,
            PendingEntryKind::Symlink { target } => Self::Symlink {
                target: target.to_path_buf(),
            },
        }
    }
}

/// An entry read from archive but not yet processed.
///
/// The reader contains owned bytes for the file content, allowing
/// hash computation and file writing without lifetime issues.
pub struct PendingEntry<'a, R: Read> {
    pub original_path: PathBuf,
    pub size: u64,
    pub mode: Option<u32>,
    pub kind: PendingEntryKind<R>,
    _marker: PhantomData<&'a ()>,
}

impl<R: Read> PendingEntry<'_, R> {
    #[inline]
    fn sanitize(&self, dest: impl AsRef<Path>, options: &ExtractOptions) -> Result<PathBuf> {
        options.sanitize_path(&self.original_path, dest)
    }

    fn write_dir(path: &Path) -> Result<()> {
        if !path.exists() {
            std::fs::create_dir_all(path).map_err(|e| Error::DirectoryCreationFailed {
                path: path.to_path_buf(),
                source: e,
            })?;
        }
        Ok(())
    }

    #[cfg(unix)]
    fn write_symlink(target: &Path, link: &Path) -> Result<()> {
        use std::os::unix::fs::symlink;
        symlink(target, link).map_err(|e| Error::SymlinkCreationFailed {
            target: target.to_path_buf(),
            link: link.to_path_buf(),
            source: e,
        })
    }

    #[cfg(windows)]
    fn write_symlink(target: &Path, link: &Path) -> Result<()> {
        use std::os::windows::fs;
        let is_dir_target = target.is_dir() || target.to_string_lossy().ends_with('/');
        if is_dir_target {
            fs::symlink_dir(target, link).map_err(|e| Error::SymlinkCreationFailed {
                target: target.to_path_buf(),
                link: link.to_path_buf(),
                source: e,
            })
        } else {
            fs::symlink_file(target, link).map_err(|e| Error::SymlinkCreationFailed {
                target: target.to_path_buf(),
                link: link.to_path_buf(),
                source: e,
            })
        }
    }

    /// Write an entry to disk with path sanitization and return the target path.
    /// This method avoids double reading by working directly with the original reader.
    #[inline]
    fn write(mut self, dest: impl AsRef<Path>, options: &ExtractOptions) -> Result<Entry> {
        let sanitized = self.sanitize(dest, options)?;
        let hash = match &mut self.kind {
            PendingEntryKind::File(reader) => {
                if let Some(parent) = sanitized.parent() {
                    Self::write_dir(parent)?;
                }

                let mut file =
                    std::fs::File::create(&sanitized).map_err(|e| Error::ExtractionFailed {
                        path: sanitized.to_path_buf(),
                        source: e,
                    })?;

                std::io::copy(reader, &mut file)?;
                options.hash_strategy.compute(&mut *reader)?
            }
            PendingEntryKind::Directory => {
                Self::write_dir(&sanitized)?;
                None
            }
            PendingEntryKind::Symlink { target } => {
                Self::write_symlink(target, &sanitized)?;
                None
            }
        };

        options.perm_strategy.apply_to_path(&sanitized, self.mode)?;

        Ok(Entry::transit(self, sanitized, hash))
    }
}

pub enum PendingEntryKind<R: Read> {
    File(R),
    Directory,
    Symlink { target: PathBuf },
}

/// Extraction results.
pub struct Extracted {
    pub entries: Vec<Entry>,
    pub total_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct ArchiveReport {
    pub format: format::ArchiveFormat,
    pub entry_count: usize,
    pub total_bytes: u64,
    pub entries: Vec<Entry>,
}

impl ArchiveReport {
    pub fn new(format: format::ArchiveFormat, extracted: Extracted) -> Self {
        Self {
            format,
            entry_count: extracted.entries.len(),
            total_bytes: extracted.total_bytes,
            entries: extracted.entries,
        }
    }
}

/// Main extraction pipeline.
///
/// Processes entries from the source, sanitizes paths, writes files to disk,
/// computes hashes if requested, applies permissions, and reports progress.
///
/// This implementation avoids double reading by working directly with the original readers.
pub fn extract<S: EntrySource>(
    source: &mut S,
    destination: impl AsRef<Path>,
    options: &ExtractOptions,
) -> Result<Extracted> {
    let mut entries = Vec::new();
    let mut total_bytes = 0u64;
    let mut bytes_processed = 0u64;

    while let Some(pending) = source.next_entry() {
        let pending = pending?;
        bytes_processed += pending.size;
        total_bytes += pending.size;

        let entry = pending.write(destination.as_ref(), options)?;

        // Report progress if callback is provided
        if let Some(ref callback) = options.on_progress {
            let percentage = options.expected_total_bytes.and_then(|expected| {
                if expected > 0 {
                    Some((bytes_processed as f32 / expected as f32) * 100.0)
                } else {
                    None
                }
            });

            callback(options::Progress {
                bytes_processed,
                total_bytes: Some(total_bytes),
                percentage,
                current_file: Some(entry.original_path.clone()),
            });
        }

        entries.push(entry);
    }

    Ok(Extracted {
        entries,
        total_bytes,
    })
}

/// Extract archive with automatic format detection.
///
/// This function detects the archive format from the reader, rewinds it,
/// and then extracts all entries to the specified destination directory.
pub fn extract_from_reader<R: Read + Seek + Send + Sync>(
    mut reader: R,
    destination: &Path,
    options: &ExtractOptions,
) -> Result<ArchiveReport> {
    let format = format::detect_from_reader(&mut reader)?.ok_or(Error::UnsupportedFormat)?;
    reader.rewind()?;

    match format {
        format::ArchiveFormat::Zip => {
            let mut source = ZipSource::new(reader)?;
            let results = extract(&mut source, destination, options)?;
            Ok(ArchiveReport::new(format, results))
        }
        format::ArchiveFormat::Tar(codec) => {
            let mut archive = TarArchive::new(reader, codec)?;
            let results = extract(&mut archive.entries()?, destination, options)?;
            Ok(ArchiveReport::new(format, results))
        }
    }
}

/// Extract to workspace for atomic commit.
pub fn extract_to_workspace<R: Read + Seek + Send + Sync>(
    reader: R,
    destination: &Path,
    options: ExtractOptions,
) -> Result<WorkspaceExtraction> {
    let temp_dir = tempfile::Builder::new()
        .prefix("pulith-archive-")
        .tempdir()
        .map_err(|e| Error::ExtractionFailed {
            path: destination.to_path_buf(),
            source: e,
        })?;

    let workspace = Workspace::new(temp_dir.path(), destination).map_err(Error::from)?;

    let report = extract_from_reader(reader, temp_dir.path(), &options)?;

    Ok(WorkspaceExtraction { workspace, report })
}

pub struct WorkspaceExtraction {
    workspace: Workspace,
    report: ArchiveReport,
}

impl WorkspaceExtraction {
    pub fn commit(self) -> Result<ArchiveReport> {
        self.workspace.commit()?;
        Ok(self.report)
    }

    pub fn abort(self) {
        drop(self.workspace);
    }

    pub fn report(&self) -> &ArchiveReport {
        &self.report
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;

    use super::*;

    #[test]
    fn extract_from_reader_invalid_format() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        let cursor = Cursor::new(data);
        let result = extract_from_reader(cursor, Path::new("/tmp"), &ExtractOptions::default());
        assert!(result.is_err());
    }
}
