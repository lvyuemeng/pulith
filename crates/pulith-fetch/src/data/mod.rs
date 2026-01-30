//! Immutable data types for HTTP fetching operations.
//!
//! This module contains all the configuration types, options, and progress
//! tracking structures that are used throughout the crate. These types are
//! immutable and designed to be passed between functions without mutation.

pub mod options;
pub mod progress;
pub mod sources;
pub mod extended_progress;

pub use options::{FetchOptions, FetchPhase};
pub use progress::Progress;
pub use sources::{DownloadSource, MultiSourceOptions, SourceSelectionStrategy, SourceType};
pub use extended_progress::{ExtendedProgress, ProgressReporter};
