//! Content verification primitives for downloaded artifacts.
//!
//! Zero-copy streaming verification for downloaded artifacts, ensuring integrity
//! without additional memory overhead.
//!
//! # Design Principles
//!
//! - **Zero-Copy Verification**: CPU cache touches bytes only once (hashing + I/O)
//! - **Composability**: Generic over any `Hasher` trait implementation
//! - **Extensibility**: Built on `digest::Digest` for broad algorithm support
//! - **Error Handling**: Concrete error types using `thiserror`
//!
//! # Key Features
//!
//! - **Zero-copy verification**: CPU cache touches bytes only once (for both hashing and writing)
//! - **Incremental**: Computes digests as data streams through
//! - **Extensible**: Minimal `Hasher` trait allows custom implementations
//! - **Thread-safe**: All public types implement `Send + Sync`
//!
//! # Example
//!
//! ```
//! use pulith_verify::{VerifiedReader, Sha256Hasher, VerifyError};
//! use std::fs::File;
//! use std::io::{self, Read};
//!
//! fn verify_artifact(path: &str, expected_hash_hex: &str) -> Result<(), VerifyError> {
//!     let expected = hex::decode(expected_hash_hex)?;
//!     let file = File::open(path)?;
//!     let hasher = Sha256Hasher::new();
//!     let mut reader = VerifiedReader::new(file, hasher);
//!
//!     let mut buffer = vec![0; 8192];
//!     loop {
//!         match reader.read(&mut buffer) {
//!             Ok(0) => break,
//!             Ok(_) => {},
//!             Err(e) => return Err(VerifyError::Io(e)),
//!         }
//!     }
//!
//!     reader.finish(&expected)?;
//!     Ok(())
//! }
//! ```

pub use self::error::{Result, VerifyError};
pub use self::hasher::{DigestHasher, Hasher};
pub use self::reader::VerifiedReader;

#[cfg(feature = "sha256")]
pub use self::hasher::Sha256Hasher;

#[cfg(feature = "blake3")]
pub use self::hasher::Blake3Hasher;

#[cfg(feature = "sha3")]
pub use self::hasher::Sha3_256Hasher;

mod error;
mod hasher;
mod reader;
