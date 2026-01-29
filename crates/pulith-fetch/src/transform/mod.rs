//! Stream transformations for HTTP fetching.
//!
//! This module contains types and functions for transforming data streams,
//! including decompression, encryption/decryption, and other streaming
//! operations that can be applied during the fetch process.

mod decompress;
mod verify;

pub use decompress::{StreamTransform, TransformError};