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
pub use options::{ExtractOptions, SanitizedPath};
pub use workspace::{WorkspaceExtraction, extract_to_workspace};

pub mod entry;
mod error;
pub mod extract;
mod format;
pub mod options;
mod workspace;
