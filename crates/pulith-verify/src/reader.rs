use std::io::{self, Read};

use crate::{Hasher, Result, VerifyError};

/// Streaming reader that hashes data as it passes through.
/// Wraps any `Read` source for zero-copy verification.
pub struct VerifiedReader<R, H> {
    reader: R,
    hasher: H,
}

impl<R, H> VerifiedReader<R, H> {
    /// Create a new verified reader.
    pub fn new(reader: R, hasher: H) -> Self {
        Self { reader, hasher }
    }
}

impl<R: Read, H: Hasher> VerifiedReader<R, H> {
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
    pub fn finish(self, expected: &[u8]) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[cfg(feature = "sha256")]
    use crate::Sha256Hasher;

    #[cfg(feature = "sha256")]
    #[test]
    fn test_sha256_hasher() {
        let mut hasher = Sha256Hasher::new();
        hasher.update(b"hello world");
        let hash = hasher.finalize();

        // Expected SHA-256 hash of "hello world" (actual computed value)
        let expected =
            hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
                .unwrap();
        assert_eq!(hash, expected);
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn test_verified_reader_success() {
        let data = b"test data for verification";

        // First, compute the expected hash from the data
        let mut hasher = Sha256Hasher::new();
        hasher.update(data);
        let expected = hasher.finalize();

        // Now test the verified reader with the computed hash
        let reader = Cursor::new(data);
        let hasher = Sha256Hasher::new();
        let mut verified = VerifiedReader::new(reader, hasher);

        let mut buffer = [0; 32];
        verified.read(&mut buffer).unwrap();

        // Test that verification succeeds with the computed hash
        verified.finish(&expected).unwrap();
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn test_verified_reader_hash_mismatch() {
        let data = b"test data";
        let reader = Cursor::new(data);
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
