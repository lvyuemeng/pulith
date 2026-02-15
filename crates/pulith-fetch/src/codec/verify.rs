//! Stream verification functionality.
//!
//! This module provides stream transformation for verifying
//! downloaded content integrity using various checksum algorithms.

use crate::error::{Error, Result};
use pulith_verify::{Hasher, Sha256Hasher};

/// Supported hash algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256 algorithm
    Sha256,
    /// SHA-384 algorithm (not yet implemented)
    Sha384,
    /// SHA-512 algorithm (not yet implemented)
    Sha512,
    /// SHA-1 algorithm (not yet implemented)
    Sha1,
    /// MD5 algorithm (not yet implemented)
    Md5,
}

impl HashAlgorithm {
    /// Get the digest length in bytes for this algorithm.
    pub fn digest_length(&self) -> usize {
        match self {
            HashAlgorithm::Sha256 => 32,
            HashAlgorithm::Sha384 => 48,
            HashAlgorithm::Sha512 => 64,
            HashAlgorithm::Sha1 => 20,
            HashAlgorithm::Md5 => 16,
        }
    }

    /// Get the string representation of this algorithm.
    pub fn as_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Sha384 => "sha384",
            HashAlgorithm::Sha512 => "sha512",
            HashAlgorithm::Sha1 => "sha1",
            HashAlgorithm::Md5 => "md5",
        }
    }
}

/// Checksum verification configuration.
#[derive(Debug, Clone)]
pub struct ChecksumConfig {
    /// The hash algorithm to use
    pub algorithm: HashAlgorithm,
    /// Expected checksum value (hex-encoded)
    pub expected: String,
}

impl ChecksumConfig {
    /// Create a new checksum configuration.
    pub fn new(algorithm: HashAlgorithm, expected: &str) -> Self {
        Self {
            algorithm,
            expected: expected.to_lowercase(),
        }
    }

    /// Parse checksum from string with algorithm prefix (e.g., "sha256:abc123").
    pub fn from_string(checksum_str: &str) -> Result<Self> {
        if let Some((algo, hash)) = checksum_str.split_once(':') {
            let algorithm = match algo.to_lowercase().as_str() {
                "sha256" => HashAlgorithm::Sha256,
                "sha384" => HashAlgorithm::Sha384,
                "sha512" => HashAlgorithm::Sha512,
                "sha1" => HashAlgorithm::Sha1,
                "md5" => HashAlgorithm::Md5,
                _ => {
                    return Err(Error::InvalidState(format!(
                        "Unsupported hash algorithm: {}",
                        algo
                    )))
                }
            };

            if hash.len() != algorithm.digest_length() * 2 {
                return Err(Error::InvalidState(format!(
                    "Invalid checksum length for {}: expected {}, got {}",
                    algo,
                    algorithm.digest_length() * 2,
                    hash.len()
                )));
            }

            Ok(Self::new(algorithm, hash))
        } else {
            // Default to SHA256 if no algorithm specified
            if checksum_str.len() != 64 {
                return Err(Error::InvalidState(
                    "Invalid checksum length for SHA256: expected 64 characters".to_string(),
                ));
            }
            Ok(Self::new(HashAlgorithm::Sha256, checksum_str))
        }
    }
}

/// Stream verifier for checksum verification.
pub struct StreamVerifier<H: Hasher> {
    hasher: Option<H>,
    config: ChecksumConfig,
    bytes_processed: usize,
    finalized: bool,
}

impl StreamVerifier<Sha256Hasher> {
    /// Create a new stream verifier with the given configuration.
    pub fn new(config: ChecksumConfig) -> Result<Self> {
        let hasher = match config.algorithm {
            HashAlgorithm::Sha256 => Some(Sha256Hasher::new()),
            _ => {
                return Err(Error::InvalidState(format!(
                    "Hash algorithm {:?} not yet implemented",
                    config.algorithm
                )))
            }
        };

        Ok(Self {
            hasher,
            config,
            bytes_processed: 0,
            finalized: false,
        })
    }
}

impl<H: Hasher> StreamVerifier<H> {
    /// Update the verifier with new data.
    pub fn update(&mut self, data: &[u8]) -> Result<()> {
        if self.finalized {
            return Err(Error::InvalidState(
                "Verifier already finalized".to_string(),
            ));
        }

        if let Some(ref mut hasher) = self.hasher {
            hasher.update(data);
        }
        self.bytes_processed += data.len();
        Ok(())
    }

    /// Finalize verification and check if the checksum matches.
    pub fn finalize(&mut self) -> Result<bool> {
        if self.finalized {
            return Err(Error::InvalidState(
                "Verifier already finalized".to_string(),
            ));
        }

        if let Some(hasher) = self.hasher.take() {
            let actual = hasher.finalize();
            let actual_hex = hex::encode(actual);
            self.finalized = true;
            Ok(actual_hex == self.config.expected)
        } else {
            Err(Error::InvalidState("No hasher available".to_string()))
        }
    }

    /// Get the number of bytes processed so far.
    pub fn bytes_processed(&self) -> usize {
        self.bytes_processed
    }

    /// Get the configured algorithm.
    pub fn algorithm(&self) -> HashAlgorithm {
        self.config.algorithm
    }

    /// Get the expected checksum.
    pub fn expected_checksum(&self) -> &str {
        &self.config.expected
    }

    /// Check if the verifier has been finalized.
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }
}

/// Multiple checksum verifier for verifying against multiple algorithms.
pub struct MultiVerifier {
    verifiers: Vec<StreamVerifier<Sha256Hasher>>,
    require_all: bool,
}

impl MultiVerifier {
    /// Create a new multi-verifier.
    ///
    /// If `require_all` is true, all checksums must match.
    /// If false, at least one checksum must match.
    pub fn new(configs: Vec<ChecksumConfig>, require_all: bool) -> Result<Self> {
        let verifiers: Result<Vec<_>> = configs.into_iter().map(StreamVerifier::new).collect();

        Ok(Self {
            verifiers: verifiers?,
            require_all,
        })
    }

    /// Update all verifiers with new data.
    pub fn update(&mut self, data: &[u8]) -> Result<()> {
        for verifier in &mut self.verifiers {
            verifier.update(data)?;
        }
        Ok(())
    }

    /// Finalize verification and check if checksums match.
    pub fn finalize(&mut self) -> Result<bool> {
        let mut results = Vec::new();
        for verifier in &mut self.verifiers {
            results.push(verifier.finalize()?);
        }

        if self.require_all {
            Ok(results.iter().all(|&r| r))
        } else {
            Ok(results.iter().any(|&r| r))
        }
    }

    /// Get the number of verifiers.
    pub fn verifier_count(&self) -> usize {
        self.verifiers.len()
    }
}

/// Convenience function to verify data in one go.
pub fn verify_checksum(data: &[u8], config: &ChecksumConfig) -> Result<bool> {
    let mut verifier = StreamVerifier::new(config.clone())?;
    verifier.update(data)?;
    verifier.finalize()
}

/// Convenience function to verify data with multiple checksums.
pub fn verify_multiple_checksums(
    data: &[u8],
    configs: Vec<ChecksumConfig>,
    require_all: bool,
) -> Result<bool> {
    let mut verifier = MultiVerifier::new(configs, require_all)?;
    verifier.update(data)?;
    verifier.finalize()
}

/// Parse multiple checksums from a string.
///
/// Supports formats like:
/// - "sha256:abc123 sha512:def456"
/// - "sha256:abc123\nsha512:def456"
pub fn parse_multiple_checksums(checksums_str: &str) -> Result<Vec<ChecksumConfig>> {
    let mut configs = Vec::new();

    for line in checksums_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split by whitespace to handle multiple checksums on one line
        for checksum_str in line.split_whitespace() {
            configs.push(ChecksumConfig::from_string(checksum_str)?);
        }
    }

    Ok(configs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_config_creation() {
        let config = ChecksumConfig::new(HashAlgorithm::Sha256, "abc123");
        assert_eq!(config.algorithm, HashAlgorithm::Sha256);
        assert_eq!(config.expected, "abc123");
    }

    #[test]
    fn test_checksum_config_from_string() {
        // With algorithm prefix - need 64 chars for SHA256
        let config = ChecksumConfig::from_string(
            "sha256:abc123def456abc123def456abc123def456abc123def456abc123def4567890",
        )
        .unwrap();
        assert_eq!(config.algorithm, HashAlgorithm::Sha256);
        assert_eq!(
            config.expected,
            "abc123def456abc123def456abc123def456abc123def456abc123def4567890"
        );

        // Without algorithm prefix (defaults to SHA256) - need 64 chars
        let config = ChecksumConfig::from_string(
            "abc123def456abc123def456abc123def456abc123def456abc123def4567890",
        )
        .unwrap();
        assert_eq!(config.algorithm, HashAlgorithm::Sha256);
        assert_eq!(
            config.expected,
            "abc123def456abc123def456abc123def456abc123def456abc123def4567890"
        );
    }

    #[test]
    fn test_checksum_config_invalid_algorithm() {
        let result = ChecksumConfig::from_string("invalid:abc123");
        assert!(result.is_err());
    }

    #[test]
    fn test_checksum_config_invalid_length() {
        // Invalid SHA256 length
        let result = ChecksumConfig::from_string("sha256:abc");
        assert!(result.is_err());

        // Invalid default length
        let result = ChecksumConfig::from_string("abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_verifier() {
        let data = b"Hello, World!";
        let config = ChecksumConfig::new(
            HashAlgorithm::Sha256,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f",
        );

        let mut verifier = StreamVerifier::new(config).unwrap();
        verifier.update(data).unwrap();
        let result = verifier.finalize().unwrap();

        assert!(result);
        assert_eq!(verifier.bytes_processed(), data.len());
    }

    #[test]
    fn test_stream_verifier_wrong_checksum() {
        let data = b"Hello, World!";
        let config = ChecksumConfig::new(HashAlgorithm::Sha256, "wrong_checksum");

        let mut verifier = StreamVerifier::new(config).unwrap();
        verifier.update(data).unwrap();
        let result = verifier.finalize().unwrap();

        assert!(!result);
    }

    #[test]
    fn test_stream_verifier_partial_updates() {
        let data = b"Hello, World!";
        let config = ChecksumConfig::new(
            HashAlgorithm::Sha256,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f",
        );

        let mut verifier = StreamVerifier::new(config).unwrap();

        // Update in chunks
        verifier.update(&data[..5]).unwrap();
        verifier.update(&data[5..]).unwrap();

        let result = verifier.finalize().unwrap();
        assert!(result);
    }

    #[test]
    fn test_multi_verifier_all_required() {
        let data = b"Hello, World!";
        let configs = vec![ChecksumConfig::new(
            HashAlgorithm::Sha256,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f",
        )];

        let mut verifier = MultiVerifier::new(configs, true).unwrap();
        verifier.update(data).unwrap();
        let result = verifier.finalize().unwrap();

        assert!(result);
    }

    #[test]
    fn test_multi_verifier_any_required() {
        let data = b"Hello, World!";
        let configs = vec![ChecksumConfig::new(
            HashAlgorithm::Sha256,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f",
        )];

        let mut verifier = MultiVerifier::new(configs, false).unwrap();
        verifier.update(data).unwrap();
        let result = verifier.finalize().unwrap();

        assert!(result);
    }

    #[test]
    fn test_convenience_functions() {
        let data = b"Hello, World!";
        let config = ChecksumConfig::new(
            HashAlgorithm::Sha256,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f",
        );

        // Test single checksum verification
        let result = verify_checksum(data, &config).unwrap();
        assert!(result);

        // Test multiple checksum verification
        let configs = vec![config.clone()];
        let result = verify_multiple_checksums(data, configs, true).unwrap();
        assert!(result);
    }

    #[test]
    fn test_parse_multiple_checksums() {
        let input = "sha256:abc123def456abc123def456abc123def456abc123def456abc123def4567890";

        let configs = parse_multiple_checksums(input).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].algorithm, HashAlgorithm::Sha256);
        assert_eq!(
            configs[0].expected,
            "abc123def456abc123def456abc123def456abc123def456abc123def4567890"
        );
    }
}

#[test]
fn test_empty_data() {
    let data = b"";
    let config = ChecksumConfig::new(
        HashAlgorithm::Sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );

    let mut verifier = StreamVerifier::new(config).unwrap();
    verifier.update(data).unwrap();
    let result = verifier.finalize().unwrap();

    assert!(result);
}

#[test]
fn test_large_data() {
    let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    // Compute the actual SHA256 hash
    let mut hasher = Sha256Hasher::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    let expected = hex::encode(hash);

    let config = ChecksumConfig::new(HashAlgorithm::Sha256, &expected);

    let mut verifier = StreamVerifier::new(config).unwrap();
    verifier.update(&data).unwrap();
    let result = verifier.finalize().unwrap();

    assert!(result);
}

#[test]
fn test_unsupported_algorithm() {
    let config = ChecksumConfig::new(HashAlgorithm::Sha512, "abc123");
    let result = StreamVerifier::new(config);
    assert!(result.is_err());
}
