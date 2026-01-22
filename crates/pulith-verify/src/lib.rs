//! Content verification primitives for downloaded artifacts.
//!
//! Provides incremental hashing and verification without enforcing specific hash
//! algorithms or verification policies. Enables single-pass verification during
//! data movement, minimizing CPU cache churn.
//!
//! # Key Features
//!
//! - **Zero-copy verification**: CPU cache touches bytes only once (for both hashing and writing)
//! - **Incremental**: Computes digests as data streams through
//! - **Extensible**: Minimal `Hasher` trait allows custom implementations
//!
//! # Example
//!
//! ```
//! use pulith_verify::{VerifiedReader, Sha256Hasher};
//!
//! let data = b"hello world";
//! let expected = Sha256Hasher::digest(b"hello world");
//!
//! let reader = VerifiedReader::new(&data[..], Sha256Hasher::new());
//! let mut buffer = Vec::new();
//! std::io::copy(&mut reader.to_slice(), &mut buffer).unwrap();
//!
//! reader.finish(&expected).unwrap();
//! ```

pub use self::error::{Result, VerificationError};
pub use self::hasher::{DigestHasher, Hasher};
pub use self::reader::VerifiedReader;

#[cfg(feature = "sha256")]
pub use self::hasher::Sha256Hasher;

#[cfg(feature = "blake3")]
pub use self::hasher::Blake3Hasher;

mod error;
mod hasher;
mod reader;
