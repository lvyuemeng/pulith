# pulith-verify

Content verification primitives. Zero-copy streaming verification.

## API

```rust
pub trait Hasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> Vec<u8>;
}

// VerifiedReader: wraps any Read, hashes as bytes pass through
VerifiedReader<R, H>::new(reader, hasher);
reader.read(&mut buf) -> io::Result<usize>;  // Hashes in-place
reader.finish(expected) -> Result<()>;       // Verify
```

## Built-in Hashers

```rust
#[cfg(feature = "sha256")]
Sha256Hasher(sha2::Sha256);  // default

#[cfg(feature = "blake3")]
Blake3Hasher(blake3::Hasher);
```

## Example

```rust
use pulith_verify::{VerifiedReader, Sha256Hasher};

let expected = hex::decode("...")?;
let hasher = Sha256Hasher::new();
let mut reader = VerifiedReader::new(file, hasher);

io::copy(&mut reader, &mut dest)?;
reader.finish(&expected)?;
```

## Dependencies

```
thiserror

[features]
default = ["sha256"]
sha256 = ["dep:sha2", "dep:digest"]
blake3 = ["dep:blake3"]
```

## Relationship

```
pulith-verify
    ├── Hasher trait
    ├── VerifiedReader<R, H>
    └── DigestHasher<D>

Used by: pulith-fetch, pulith-archive
```
