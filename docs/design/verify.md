# pulith-verify

Content verification primitives. Zero-copy streaming verification for downloaded artifacts, ensuring integrity without additional memory overhead.

## Design Principles

- **Zero-Copy Verification**: CPU cache touches bytes only once (hashing + I/O), following F3 (Pure Core, Impure Edge) and E3 (Batch at Boundaries) from [AGENT.md](../AGENT.md).
- **Composability**: Generic over any `Hasher` trait implementation, enabling custom algorithms (F5 - Composition Over Orchestration).
- **Extensibility**: Built on `digest::Digest` for broad algorithm support, with `DigestHasher<D>` as the primary generic wrapper.
- **Error Handling**: Concrete error types using `thiserror`, with transparent external errors following [AGENT.md](../AGENT.md) Error Handling guidelines.

## API

### Core Traits

```rust
/// Minimal hasher interface for streaming verification.
/// Implementations must be Send for cross-thread safety.
pub trait Hasher: Send {
    /// Update the hash with new data.
    fn update(&mut self, data: &[u8]);

    /// Finalize and return the hash digest.
    /// Consumes self to prevent reuse.
    fn finalize(self) -> Vec<u8>;
}

/// Error types for verification operations.
/// 
/// Follows the error handling patterns specified in [AGENT.md](../AGENT.md).
#[derive(Error, Debug)]
pub enum VerifyError {
    /// Hash mismatch between expected and actual digest
    #[error("hash mismatch: expected {expected:?}, got {actual:?}")]
    HashMismatch { 
        /// The expected hash digest
        expected: Vec<u8>, 
        /// The actual computed hash digest
        actual: Vec<u8> 
    },

    /// I/O error during verification process
    #[error("I/O error during verification: {0}")]
    Io(#[from] std::io::Error),

    /// Hexadecimal decoding error when parsing expected hash
    #[error("hex decoding error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}
```

### VerifiedReader

```rust
/// Streaming reader that hashes data as it passes through.
/// Wraps any `Read` source for zero-copy verification.
pub struct VerifiedReader<R, H> {
    reader: R,
    hasher: H,
}

impl<R, H> VerifiedReader<R, H>
where
    R: Read,
    H: Hasher,
{
    /// Create a new verified reader.
    pub fn new(reader: R, hasher: H) -> Self {
        Self { reader, hasher }
    }

    /// Read data, hashing it in-place.
    /// Delegates to inner reader.
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf)?;
        if n > 0 {
            self.hasher.update(&buf[..n]);
        }
        Ok(n)
    }

    /// Finalize verification against expected hash.
    /// Returns error on mismatch.
    pub fn finish(self, expected: &[u8]) -> Result<(), VerifyError> {
        let actual = self.hasher.finalize();
        if actual == expected {
            Ok(())
        } else {
            Err(VerifyError::HashMismatch {
                expected: expected.to_vec(),
                actual,
            })
        }
    }
}
```

### DigestHasher

```rust
/// Generic hasher wrapper for any `digest::Digest` implementation.
/// Provides the primary way to use standard hashing algorithms.
/// Enables composability with external crates like `sha2`, `sha3`, `blake3`.
pub struct DigestHasher<D: digest::Digest>(D);

impl<D: digest::Digest + Send> DigestHasher<D> {
    /// Create from a digest instance.
    pub fn new(digest: D) -> Self {
        Self(digest)
    }
}

impl<D: digest::Digest> Hasher for DigestHasher<D> {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(self) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}
```

## Built-in Hashers

Convenience constructors for common algorithms. All use `DigestHasher<D>` internally for consistency.

```rust
#[cfg(feature = "sha256")]
pub type Sha256Hasher = DigestHasher<sha2::Sha256>;

#[cfg(feature = "sha256")]
impl Sha256Hasher {
    /// Create a new SHA-256 hasher instance.
    pub fn new() -> Self {
        DigestHasher::new(sha2::Sha256::new())
    }
}

#[cfg(feature = "blake3")]
pub type Blake3Hasher = DigestHasher<blake3::Hasher>;

#[cfg(feature = "blake3")]
impl Blake3Hasher {
    /// Create a new Blake3 hasher instance.
    pub fn new() -> Self {
        DigestHasher::new(blake3::Hasher::new())
    }
}

#[cfg(feature = "sha3")]
pub type Sha3_256Hasher = DigestHasher<sha3::Sha3_256>;

#[cfg(feature = "sha3")]
impl Sha3_256Hasher {
    /// Create a new SHA3-256 hasher instance.
    pub fn new() -> Self {
        DigestHasher::new(sha3::Sha3_256::new())
    }
}
```

## Example

```rust
use pulith_verify::{VerifiedReader, Sha256Hasher, VerifyError};
use std::fs::File;
use std::io::{self, Read};

/// Verify a downloaded artifact against its expected SHA-256 hash.
/// 
/// This example demonstrates the zero-copy verification pattern:
/// - Data is read from the file
/// - Hash is computed during the read operation
/// - Final verification is performed atomically
fn verify_artifact(path: &str, expected_hash_hex: &str) -> Result<(), VerifyError> {
    // Decode expected hash from hexadecimal string
    let expected = hex::decode(expected_hash_hex)
        .map_err(|e| VerifyError::HexDecode(e))?;
    
    // Open the file for reading
    let file = File::open(path)
        .map_err(|e| VerifyError::Io(e))?;
    
    // Create hasher and verified reader
    let hasher = Sha256Hasher::new();
    let mut reader = VerifiedReader::new(file, hasher);

    // Stream data while computing hash (zero-copy)
    let mut buffer = vec![0; 8192]; // 8KB buffer
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                // Process n bytes if needed
                // Hash is automatically computed during read()
            }
            Err(e) => return Err(VerifyError::Io(e)),
        }
    }

    // Final verification - this will return an error if hashes don't match
    reader.finish(&expected)?;
    
    Ok(())
}

// Usage example
fn main() -> Result<(), Box<dyn std::error::Error>> {
    match verify_artifact("artifact.bin", "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3") {
        Ok(()) => println!("✓ Artifact verification successful!"),
        Err(e) => println!("✗ Verification failed: {}", e),
    }
    Ok(())
}
```

## Dependencies

```
thiserror = "1.0"
hex = { version = "0.4", optional = true }

[dependencies.digest]
version = "0.10"
default-features = false

[features]
default = ["sha256"]
sha256 = ["dep:sha2", "digest/std"]
blake3 = ["dep:blake3", "digest/std"]
sha3 = ["dep:sha3", "digest/std"]
```

## Relationship

```
pulith-verify
    ├── Hasher trait (core abstraction)
    ├── VerifyError (error handling)
    ├── VerifiedReader<R, H> (streaming wrapper)
    └── DigestHasher<D> (generic digest adapter)

Used by: pulith-fetch (streaming downloads), pulith-archive (optional integrity checks)
```

## Testing

The crate includes comprehensive tests following [AGENT.md](../AGENT.md) testing requirements:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_sha256_hasher() {
        let mut hasher = Sha256Hasher::new();
        hasher.update(b"hello world");
        let hash = hasher.finalize();
        
        // Expected SHA-256 hash of "hello world"
        let expected = hex::decode("a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e").unwrap();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_verified_reader_success() {
        let data = b"test data for verification";
        let mut reader = Cursor::new(data);
        let hasher = Sha256Hasher::new();
        let mut verified = VerifiedReader::new(reader, hasher);
        
        let mut buffer = [0; 32];
        verified.read(&mut buffer).unwrap();
        
        // Expected hash of "test data for verification"
        let expected = hex::decode("5f2c3777c909a909226f0b3dd0c9101627fc8bb4e061dc828ca25b6f542856d8").unwrap();
        verified.finish(&expected).unwrap();
    }

    #[test]
    fn test_verified_reader_hash_mismatch() {
        let data = b"test data";
        let mut reader = Cursor::new(data);
        let hasher = Sha256Hasher::new();
        let mut verified = VerifiedReader::new(reader, hasher);
        
        let mut buffer = [0; 32];
        verified.read(&mut buffer).unwrap();
        
        // Wrong hash should cause error
        let wrong_hash = vec![0; 32];
        let result = verified.finish(&wrong_hash);
        assert!(result.is_err());
        
        if let Err(VerifyError::HashMismatch { expected, actual }) = result {
            assert_eq!(expected, vec![0; 32]);
            assert_ne!(actual, vec![0; 32]);
        } else {
            panic!("Expected HashMismatch error");
        }
    }
}
```

## Implementation Notes

- **Performance**: Zero-copy by hashing during `read()`, avoiding double buffering. The `VerifiedReader` processes data in chunks, updating the hash incrementally.
- **Thread Safety**: `Hasher` requires `Send` for potential async usage. All public types are `Send + Sync` as required by [AGENT.md](../AGENT.md).
- **Extensibility**: Add new algorithms by implementing `Hasher` or using `DigestHasher<D>` with compatible digests. The trait-based design enables easy extension.
- **Security**: No built-in key derivation; for HMAC, compose externally using the `Hasher` trait. All hash operations are performed in constant time where possible.
- **Memory Efficiency**: Uses stack-allocated buffers for small reads and heap allocation only when necessary. The `DigestHasher` wrapper has minimal overhead.
- **Error Propagation**: All I/O errors are transparently propagated using `thiserror`'s `#[from]` attribute, following [AGENT.md](../AGENT.md) error handling patterns.
