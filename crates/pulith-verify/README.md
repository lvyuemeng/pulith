# pulith-verify

Streaming content verification primitives.

## Role

`pulith-verify` verifies bytes. It should stay verification-only.

It does not own:

- fetch orchestration
- trust-policy decisions
- resource semantics

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

## How To Use It

Use this crate to stream data through a verifier while another crate decides:

- where bytes come from
- whether a failure is retriable
- what trust policy to enforce

See `docs/design/verify.md`.
