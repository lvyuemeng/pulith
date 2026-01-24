use digest::Digest;

/// Minimal hasher interface for streaming verification.
/// Implementations must be Send for cross-thread safety.
pub trait Hasher: Send {
    /// Update the hash with new data.
    fn update(&mut self, data: &[u8]);

    /// Finalize and return the hash digest.
    /// Consumes self to prevent reuse.
    fn finalize(self) -> Vec<u8>;
}

/// Generic hasher wrapper for any `digest::Digest` implementation.
/// Provides the primary way to use standard hashing algorithms.
/// Enables composability with external crates like `sha2`, `sha3`, `blake3`.
pub struct DigestHasher<D: digest::Digest + Send>(D);

impl<D: digest::Digest + Send> DigestHasher<D> {
    /// Create from a digest instance.
    pub fn from_digest(digest: D) -> Self {
        Self(digest)
    }
}

impl<D: digest::Digest + Send> Hasher for DigestHasher<D> {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(self) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}

/// Built-in hashers as type aliases and constructors for convenience.

#[cfg(feature = "sha256")]
pub type Sha256Hasher = DigestHasher<sha2::Sha256>;

#[cfg(feature = "sha256")]
impl Sha256Hasher {
    /// Create a new SHA-256 hasher instance.
    pub fn new() -> Self {
        DigestHasher::from_digest(sha2::Sha256::new())
    }
}

#[cfg(feature = "blake3")]
pub type Blake3Hasher = DigestHasher<blake3::Hasher>;

#[cfg(feature = "blake3")]
impl Blake3Hasher {
    /// Create a new Blake3 hasher instance.
    pub fn new() -> Self {
        DigestHasher::from_digest(blake3::Hasher::new())
    }
}

#[cfg(feature = "sha3")]
pub type Sha3_256Hasher = DigestHasher<sha3::Sha3_256>;

#[cfg(feature = "sha3")]
impl Sha3_256Hasher {
    /// Create a new SHA3-256 hasher instance.
    pub fn new() -> Self {
        DigestHasher::from_digest(sha3::Sha3_256::new())
    }
}
