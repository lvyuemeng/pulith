//! Archive extraction with path sanitization and transactional staging.
//!
//! # Architecture
//!
//! - `detect.rs` - Format detection
//! - `sanitize.rs` - Path sanitization (zip-slip prevention)
//! - `workspace.rs` - Transactional extraction
//! - `extract/` - Per-format implementations
//! - `data/` - Shared types
//! - `codec/` - Compression codecs
//! - `ops/` - Permission and hash operations

pub use error::{Error, Result};
pub use extract::extract_from_reader;
pub use options::ExtractOptions;
pub use sanitize::{sanitize_path_with_options, sanitize_symlink_target_with_options, SanitizedPath};
pub use workspace::{extract_to_workspace, WorkspaceExtraction};

pub mod options;
mod format;
pub mod entry;
pub mod extract;
mod error;
mod sanitize;
mod workspace;