//! Core layer: pure transformations and verification logic.

use crate::data::Sha256Hash;
use sha2::{Digest, Sha256};

pub fn verify_checksum(bytes: &[u8], expected: &Sha256Hash) -> Result<(), ChecksumError> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual = format!("{:x}", hasher.finalize());

    if expected.as_str() != actual {
        Err(ChecksumError)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_checksum_valid() {
        let data = b"hello world";
        let hash = Sha256Hash(
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
        );
        assert!(verify_checksum(data, &hash).is_ok());
    }

    #[test]
    fn verify_checksum_invalid() {
        let data = b"hello world";
        let hash = Sha256Hash(
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        );
        assert!(verify_checksum(data, &hash).is_err());
    }
}
