# pulith-verify

Streaming content verification primitives.

## Main APIs

- `VerifiedReader`
- `Hasher`
- `DigestHasher`
- `Sha256Hasher` (feature)
- `Blake3Hasher` (feature)

## Basic Usage

```rust
use pulith_verify::{Sha256Hasher, VerifiedReader};
use std::io::Read;

let data = std::io::Cursor::new(b"hello".to_vec());
let mut reader = VerifiedReader::new(data, Sha256Hasher::new());
let mut out = Vec::new();
reader.read_to_end(&mut out)?;
# Ok::<(), std::io::Error>(())
```

See `docs/design/verify.md`.
