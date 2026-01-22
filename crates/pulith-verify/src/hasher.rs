#[cfg(feature = "sha256")]
use sha2::digest::Digest;

#[cfg(feature = "blake3")]
use blake3::digest::Digest;

#[cfg(feature = "sha256")]
use sha2::digest::Digest as ShaDigest;

#[cfg(feature = "blake3")]
use blake3::digest::Digest as Blake3Digest;

pub trait Hasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> Vec<u8>;
}

#[cfg(feature = "sha256")]
pub struct Sha256Hasher(sha2::Sha256);

#[cfg(feature = "sha256")]
impl Hasher for Sha256Hasher {
    fn update(&mut self, data: &[u8]) { self.0.update(data); }
    fn finalize(self) -> Vec<u8> { self.0.finalize().to_vec() }
}

#[cfg(feature = "sha256")]
impl Default for Sha256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256Hasher {
    pub fn new() -> Self { Self(sha2::Sha256::new()) }

    pub fn digest(data: &[u8]) -> Vec<u8> { sha2::Sha256::digest(data).to_vec() }
}

#[cfg(feature = "blake3")]
pub struct Blake3Hasher(blake3::Hasher);

#[cfg(feature = "blake3")]
impl Hasher for Blake3Hasher {
    fn update(&mut self, data: &[u8]) { self.0.update(data); }
    fn finalize(self) -> Vec<u8> { self.0.finalize().as_bytes().to_vec() }
}

#[cfg(feature = "blake3")]
impl Blake3Hasher {
    pub fn new() -> Self { Self(blake3::Hasher::new()) }

    pub fn digest(data: &[u8]) -> Vec<u8> { blake3::Hasher::digest(data).as_bytes().to_vec() }
}

#[cfg(feature = "sha256")]
pub struct DigestHasher<D: ShaDigest + Send>(D);

#[cfg(feature = "sha256")]
impl<D: ShaDigest + Send> Hasher for DigestHasher<D> {
    fn update(&mut self, data: &[u8]) { self.0.update(data); }
    fn finalize(self) -> Vec<u8> { self.0.finalize().to_vec() }
}
