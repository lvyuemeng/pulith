use std::io::{self, Read};

use crate::{Hasher, Result, VerificationError};

pub struct VerifiedReader<R, H> {
    inner:  R,
    hasher: H,
}

impl<R, H> VerifiedReader<R, H> {
    pub fn new(inner: R, hasher: H) -> Self { Self { inner, hasher } }

    pub fn into_inner(self) -> (R, H) { (self.inner, self.hasher) }

    pub fn hasher(&self) -> &H { &self.hasher }
}

impl<R, H: Hasher> VerifiedReader<R, H> {
    pub fn finish(self, expected: &[u8]) -> Result<()> {
        let actual = self.hasher.finalize();
        if actual.as_slice() == expected {
            Ok(())
        } else {
            Err(VerificationError::Mismatch {
                expected: expected.to_vec(),
                actual,
            })
        }
    }
}

impl<R: Read, H: Hasher> Read for VerifiedReader<R, H> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sha256")]
    #[test]
    fn test_verified_reader_computes_hash() {
        use crate::Sha256Hasher;

        let data = b"hello world";
        let mut reader = VerifiedReader::new(&data[..], Sha256Hasher::new());
        let mut buffer = Vec::new();
        std::io::copy(&mut reader, &mut buffer).unwrap();

        let expected = Sha256Hasher::digest(b"hello world");
        reader.finish(&expected).unwrap();
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn test_verified_reader_detects_mismatch() {
        use crate::Sha256Hasher;

        let data = b"hello world";
        let mut reader = VerifiedReader::new(&data[..], Sha256Hasher::new());
        let mut buffer = Vec::new();
        std::io::copy(&mut reader, &mut buffer).unwrap();

        let wrong_hash = vec![0u8; 32];
        assert!(matches!(
            reader.finish(&wrong_hash),
            Err(VerificationError::Mismatch { .. })
        ));
    }

    #[cfg(feature = "blake3")]
    #[test]
    fn test_blake3_hasher() {
        use crate::Blake3Hasher;

        let data = b"test data";
        let expected = Blake3Hasher::digest(b"test data");

        let mut reader = VerifiedReader::new(&data[..], Blake3Hasher::new());
        let mut buffer = Vec::new();
        std::io::copy(&mut reader, &mut buffer).unwrap();

        reader.finish(&expected).unwrap();
    }

    #[test]
    fn test_custom_hasher() {
        struct CountingHasher {
            bytes: usize,
        }

        impl Hasher for CountingHasher {
            fn update(&mut self, data: &[u8]) { self.bytes += data.len(); }
            fn finalize(self) -> Vec<u8> { self.bytes.to_le_bytes().to_vec() }
        }

        let data = b"test data";
        let mut reader = VerifiedReader::new(&data[..], CountingHasher { bytes: 0 });
        let mut buffer = Vec::new();
        std::io::copy(&mut reader, &mut buffer).unwrap();

        assert!(reader.finish(&[]).is_err());
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn test_hasher_accessor() {
        use crate::Sha256Hasher;

        let data = b"test data";
        let mut reader = VerifiedReader::new(&data[..], Sha256Hasher::new());
        let _ = reader.hasher();
        let mut buffer = Vec::new();
        std::io::copy(&mut reader, &mut buffer).unwrap();
    }
}
