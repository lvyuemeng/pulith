//! Stream transformations for HTTP fetching.
//!
//! This module contains types and functions for transforming data streams,
//! including decompression, encryption/decryption, and other streaming
//! operations that can be applied during the fetch process.

mod cache;
mod decompress;
mod signature;
mod verify;

pub use cache::{
    CacheControl, CacheEntry, CacheError, CacheStats, CacheValidation, ConditionalHeaders,
    HttpCache,
};
pub use decompress::{StreamTransform, TransformError};
pub use signature::{
    verify_signature, KeyPurpose, KeyUsage, MockVerifier, PublicKey, PublicKeyFormat, Signature,
    SignatureAlgorithm, SignatureConfig, SignatureFormat, SignatureManager, SignatureVerifier,
};
pub use verify::{
    parse_multiple_checksums, verify_checksum, verify_multiple_checksums, ChecksumConfig,
    MultiVerifier, StreamVerifier,
};
