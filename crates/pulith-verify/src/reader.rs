use std::io::{self, Read};

use crate::{Hasher, Result, VerifyError};

/// Streaming reader that hashes data as it passes through.
/// Wraps any `Read` source for zero-copy verification.
pub struct VerifiedReader<R, H> {
    reader: R,
    hasher: H,
    bytes_processed: u64,
}

impl<R, H> VerifiedReader<R, H> {
    /// Create a new verified reader.
    pub fn new(reader: R, hasher: H) -> Self {
        Self {
            reader,
            hasher,
            bytes_processed: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReceipt {
    /// Expected digest bytes supplied by the caller.
    pub expected_digest: Vec<u8>,
    /// Actual digest bytes computed from the stream.
    pub actual_digest: Vec<u8>,
    /// Number of bytes consumed from the wrapped reader.
    pub bytes_processed: u64,
}

impl<R: Read, H: Hasher> VerifiedReader<R, H> {
    /// Read data, hashing it in-place.
    /// Delegates to inner reader.
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf)?;
        if n > 0 {
            self.hasher.update(&buf[..n]);
            self.bytes_processed += n as u64;
        }
        Ok(n)
    }

    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }

    /// Finalize verification against expected hash.
    /// Returns error on mismatch.
    pub fn finish(self, expected: &[u8]) -> Result<()> {
        self.finish_with_constraints(expected, None)?;
        Ok(())
    }

    /// Finalize verification with optional stream length enforcement.
    ///
    /// # Errors
    ///
    /// Returns [`VerifyError::HashMismatch`] when digest verification fails.
    /// Returns [`VerifyError::SizeMismatch`] when `expected_bytes` is provided
    /// and differs from the consumed stream length.
    pub fn finish_with_constraints(
        self,
        expected: &[u8],
        expected_bytes: Option<u64>,
    ) -> Result<VerificationReceipt> {
        let actual = self.hasher.finalize();
        if actual != expected {
            return Err(VerifyError::HashMismatch {
                expected: expected.to_vec(),
                actual,
            });
        }

        if let Some(expected_bytes) = expected_bytes
            && self.bytes_processed != expected_bytes
        {
            return Err(VerifyError::SizeMismatch {
                expected: expected_bytes,
                actual: self.bytes_processed,
            });
        }

        Ok(VerificationReceipt {
            expected_digest: expected.to_vec(),
            actual_digest: actual,
            bytes_processed: self.bytes_processed,
        })
    }
}

/// Verifies an entire stream by reading it to EOF.
///
/// # Errors
///
/// Returns any I/O error from the wrapped reader.
/// Returns [`VerifyError::HashMismatch`] or [`VerifyError::SizeMismatch`]
/// when verification constraints fail.
pub fn verify_stream<R: Read, H: Hasher>(
    reader: R,
    hasher: H,
    expected: &[u8],
    expected_bytes: Option<u64>,
) -> Result<VerificationReceipt> {
    let mut verified = VerifiedReader::new(reader, hasher);
    let mut buffer = [0_u8; 8192];
    loop {
        let read = verified.read(&mut buffer)?;
        if read == 0 {
            break;
        }
    }
    verified.finish_with_constraints(expected, expected_bytes)
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
        let receipt = verified
            .finish_with_constraints(&expected, Some(data.len() as u64))
            .unwrap();
        assert_eq!(receipt.bytes_processed, data.len() as u64);
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

    #[cfg(feature = "sha256")]
    #[test]
    fn test_verified_reader_size_mismatch() {
        let data = b"test data";

        let mut expected_hasher = Sha256Hasher::new();
        expected_hasher.update(data);
        let expected = expected_hasher.finalize();

        let reader = Cursor::new(data);
        let hasher = Sha256Hasher::new();
        let mut verified = VerifiedReader::new(reader, hasher);

        let mut buffer = [0; 32];
        verified.read(&mut buffer).unwrap();

        let result = verified.finish_with_constraints(&expected, Some((data.len() as u64) + 1));
        assert!(matches!(
            result,
            Err(VerifyError::SizeMismatch {
                expected,
                actual
            }) if expected == (data.len() as u64) + 1 && actual == data.len() as u64
        ));
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn test_verify_stream_consumes_full_reader() {
        let data = b"stream verify content";
        let mut expected_hasher = Sha256Hasher::new();
        expected_hasher.update(data);
        let expected = expected_hasher.finalize();

        let receipt = verify_stream(
            Cursor::new(data),
            Sha256Hasher::new(),
            &expected,
            Some(data.len() as u64),
        )
        .unwrap();

        assert_eq!(receipt.bytes_processed, data.len() as u64);
    }
}
