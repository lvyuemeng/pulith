use std::io::{Read, Seek};
use std::path::Path;

use pulith_fs::workflow::Workspace;

use crate::entry::ArchiveReport;
use crate::error::Result;
use crate::extract::extract_from_reader;
use crate::options::ExtractOptions;

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
    reader: R,
    destination: &Path,
    options: ExtractOptions,
) -> Result<WorkspaceExtraction> {
    let temp_dir = tempfile::Builder::new()
        .prefix("pulith-archive-")
        .tempdir()
        .map_err(|e| crate::error::Error::ExtractionFailed {
            path: destination.to_path_buf(),
            source: e,
        })?;

    let workspace =
        Workspace::new(temp_dir.path(), destination).map_err(crate::error::Error::from)?;

    let report = extract_from_reader(reader, temp_dir.path(), &options)?;

    Ok(WorkspaceExtraction { workspace, report })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::format::ArchiveFormat;

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

        let result = extract_to_workspace(cursor, &dest, ExtractOptions::default());
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
