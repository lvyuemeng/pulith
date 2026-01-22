use std::io::{Read, Seek};
use std::path::Path;

use pulith_fs::Workspace;

use crate::data::options::ExtractionOptions;
use crate::data::report::ArchiveReport;
use crate::detect::detect_from_reader;
use crate::error::Result;
use crate::extract::extractor_for;

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

pub fn extract_to_workspace<R: Read + Seek + 'static>(
    mut reader: R,
    destination: &Path,
    options: ExtractionOptions,
) -> Result<WorkspaceExtraction> {
    let format = detect_from_reader(&mut reader)?.ok_or(crate::error::Error::UnsupportedFormat)?;

    let temp_dir = tempfile::Builder::new()
        .prefix("pulith-archive-")
        .tempdir()
        .map_err(|e| crate::error::Error::ExtractionFailed {
            path: destination.to_path_buf(),
            source: e,
        })?;

    let workspace =
        Workspace::new(temp_dir.path(), destination).map_err(|e| crate::error::Error::from(e))?;

    let extractor = extractor_for(format).ok_or(crate::error::Error::UnsupportedFormat)?;

    let report = extractor.extract(reader, temp_dir.path(), &options, Some(&workspace))?;

    Ok(WorkspaceExtraction { workspace, report })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::data::archive::ArchiveFormat;

    #[test]
    fn workspace_extraction_report_access() {
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 0,
            total_bytes: 0,
            entries: Vec::new(),
        };
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = Workspace::new(temp_dir.path(), temp_dir.path()).unwrap();
        let extraction = WorkspaceExtraction { workspace, report };
        let accessed_report = extraction.report();
        assert_eq!(accessed_report.entry_count, 0);
    }

    #[test]
    fn workspace_extraction_abort_drops_workspace() {
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 0,
            total_bytes: 0,
            entries: Vec::new(),
        };
        let temp_dir = tempfile::tempdir().unwrap();
        let staging_path = temp_dir.path().to_path_buf();
        let workspace = Workspace::new(temp_dir.path(), temp_dir.path()).unwrap();

        {
            let _extraction = WorkspaceExtraction { workspace, report };
            assert!(staging_path.exists());
        }
        assert!(!staging_path.exists());
    }

    #[test]
    fn extract_to_workspace_invalid_format() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        let cursor = Cursor::new(data);
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("dest");

        let result = extract_to_workspace(cursor, &dest, ExtractionOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn extract_to_workspace_zip_format() {
        let zip_header = [0x50, 0x4B, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00];
        let cursor = Cursor::new(zip_header);
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("dest");

        let result = extract_to_workspace(cursor, &dest, ExtractionOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn extract_to_workspace_tar_gz_format() {
        let tar_gz_header = [0x1F, 0x8B, 0x08, 0x00];
        let cursor = Cursor::new(tar_gz_header);
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("dest");

        let result = extract_to_workspace(cursor, &dest, ExtractionOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn workspace_extraction_commit_returns_report() {
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 5,
            total_bytes: 1024,
            entries: Vec::new(),
        };
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = Workspace::new(temp_dir.path(), temp_dir.path()).unwrap();
        let extraction = WorkspaceExtraction { workspace, report };
        let committed_report = extraction.commit().unwrap();
        assert_eq!(committed_report.entry_count, 5);
        assert_eq!(committed_report.total_bytes, 1024);
    }
}
