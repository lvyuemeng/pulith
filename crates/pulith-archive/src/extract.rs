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

use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use crate::entry::{Entry, EntryKind};
use crate::error::{Error, Result};
use crate::options::{self, HashStrategy, ExtractOptions};
use crate::format;
use pulith_fs::workflow::Workspace;

use crate::entry::ArchiveReport;

mod tar;
mod zip;

pub use tar::TarSource;
pub use zip::ZipSource;

/// An entry read from archive but not yet processed.
///
/// The reader contains owned bytes for the file content, allowing
/// hash computation and file writing without lifetime issues.
pub struct PendingEntry {
    pub original_path: PathBuf,
    pub size: u64,
    pub mode: Option<u32>,
    pub kind: EntryKind,
    pub reader: Option<Box<dyn Read>>,
}

/// Archive-specific entry source trait.
pub trait EntrySource {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>>;
    fn format(&self) -> format::ArchiveFormat;
}

/// Extraction results.
pub struct Extracted {
    pub entries: Vec<Entry>,
    pub total_bytes: u64,
}

/// Main extraction pipeline.
///
/// Processes entries from the source, sanitizes paths, writes files to disk,
/// computes hashes if requested, applies permissions, and reports progress.
pub fn extract<S: EntrySource>(
    source: &mut S,
    destination: impl AsRef<Path>,
    options: &ExtractOptions,
) -> Result<Extracted> {
    let mut entries = Vec::new();
    let mut total_bytes = 0u64;
    let mut bytes_processed = 0u64;

    for pending in source.entries()? {
        let mut pending = pending?;
        bytes_processed += pending.size;
        total_bytes += pending.size;

        let mut entry = Entry::new(
            pending.original_path.clone(),
            pending.size,
            pending.mode,
            pending.kind.clone(),
        );

        // Sanitize the path using the composable API
        let sanitized = options.sanitize_path(
            &entry.original_path,
            destination.as_ref(),
        )?;

        entry = entry.with_target_path(sanitized.resolved.clone());

        // Write the entry to disk
        write_entry(&mut pending, &sanitized.resolved)?;

        // Compute hash if requested and we have a reader
        if let Some(mut reader) = pending.reader {
            if options.hash_strategy != HashStrategy::None {
                let hash = options.hash_strategy.compute(&mut *reader as &mut dyn Read)?;
                if let Some(hash_value) = hash {
                    entry = entry.with_hash(hash_value);
                }
            }
        }

        // Apply permissions to the target path
        if let Some(target_path) = &entry.target_path {
            options.perm_strategy.apply_to_path(target_path, pending.mode)?;
        }

        // Report progress if callback is provided
        if let Some(ref callback) = options.on_progress {
            let percentage = options.expected_total_bytes
                .and_then(|expected| {
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

fn write_entry(pending: &mut PendingEntry, target_path: &Path) -> Result<()> {
    match &pending.kind {
        EntryKind::File => write_file(pending, target_path),
        EntryKind::Directory => ensure_directory(target_path),
        EntryKind::Symlink { target } => write_symlink(target, target_path),
    }
}

fn write_file(pending: &mut PendingEntry, target_path: &Path) -> Result<()> {
    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| Error::DirectoryCreationFailed {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
    }

    if let Some(ref mut reader) = pending.reader {
        let mut file = std::fs::File::create(target_path).map_err(|e| Error::ExtractionFailed {
            path: target_path.to_path_buf(),
            source: e,
        })?;
        std::io::copy(reader, &mut file)?;
    }

    Ok(())
}

fn ensure_directory(path: &Path) -> Result<()> {
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

/// Extract archive with automatic format detection.
///
/// This function detects the archive format from the reader, rewinds it,
/// and then extracts all entries to the specified destination directory.
pub fn extract_from_reader<R: Read + Seek>(
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
            Ok(create_report(format, results))
        }
        format::ArchiveFormat::Tar(codec) => {
            let mut source = TarSource::new(reader, codec)?;
            let results = extract(&mut source, destination, options)?;
            Ok(create_report(format, results))
        }
    }
}

/// Helper function to create an ArchiveReport from extraction results.
fn create_report(format: format::ArchiveFormat, results: Extracted) -> ArchiveReport {
    ArchiveReport {
        format,
        entry_count: results.entries.len(),
        total_bytes: results.total_bytes,
        entries: results.entries,
    }
}

/// Extract archive using explicit source.
///
/// This function allows direct use of an EntrySource implementation
/// without automatic format detection.
pub fn extract_with_source<S: EntrySource>(
    source: &mut S,
    destination: &Path,
    options: &ExtractOptions,
) -> Result<ArchiveReport> {
    let results = extract(source, destination, options)?;
    Ok(create_report(source.format(), results))
}

/// Extract to workspace for atomic commit.
pub fn extract_to_workspace<R: Read + Seek>(
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
