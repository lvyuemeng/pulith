//! Archive extraction with path sanitization and transactional staging.
//!
//! # Architecture
//!
//! - `detect.rs` - Format detection
//! - `sanitize.rs` - Path sanitization (zip-slip prevention)
//! - `workspace.rs` - Transactional extraction
//! - `extract/` - Per-format implementations
//! - `data/` - Shared types

pub use data::archive::{ArchiveFormat, Compression};
pub use data::options::{ExtractionOptions, HashStrategy, PermissionStrategy, Progress};
pub use data::report::{ArchiveReport, ExtractedEntry};
pub use detect::{detect_format, detect_from_reader};
pub use error::{Error, Result};
pub use sanitize::{sanitize_path, sanitize_symlink_target, strip_path_components};
pub use workspace::{extract_to_workspace, WorkspaceExtraction};

pub mod data;
pub mod detect;
pub mod extract;
pub mod progress {
	pub use crate::data::Progress;
}
mod error;
mod sanitize;
mod workspace;
