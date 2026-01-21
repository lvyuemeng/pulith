//! HTTP downloading with verification, progress tracking, and retry logic.
//!
//! # Architecture
//!
//! This crate follows the three-layer pattern:
//! - [`data`] - Immutable configuration and types
//! - [`core`] - Pure transformations
//! - [`effects`] - I/O operations with trait abstraction

pub use core::verify_checksum;
pub use data::{DownloadOptions, DownloadPhase, Progress, ProgressCallback, Sha256Hash};
pub use effects::Downloader;

mod core;
mod data;
mod effects;
pub mod error;
pub use error::{FetchError, ParseSha256HashError};
