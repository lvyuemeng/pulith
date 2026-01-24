//! HTTP downloading with streaming verification and atomic placement.
//!
//! # Architecture
//!
//! This crate follows the three-layer pattern:
//! - [`data`] - Immutable configuration and types
//! - [`core`] - Pure transformations
//! - [`effects`] - I/O operations with trait abstraction
//!
//! # Key Features
//!
//! - **Single-Pass**: Tee-Reader pattern hashes while streaming to avoid memory bloat
//! - **Atomic Placement**: Uses `pulith-fs::Workspace` for guaranteed cleanup on error
//! - **Streaming Verification**: Uses `pulith-verify::Hasher` for incremental hashing
//! - **Mechanism-Only**: No policy; caller handles progress UI and retry orchestration

mod core;
mod data;
mod effects;
mod error;

pub use core::{is_redirect, retry_delay};
pub use data::{FetchOptions, FetchPhase, Progress, Timeouts};
pub use effects::{BoxStream, Fetcher, HttpClient};

#[cfg(feature = "reqwest")]
pub use effects::ReqwestClient;

pub use error::FetchError;
