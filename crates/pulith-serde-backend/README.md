# pulith-serde-backend

Serialization backend contract and baseline JSON adapters for Pulith persistence boundaries.

## What This Crate Owns

`pulith-serde-backend` isolates persistence-format mechanics from semantic/workflow crates.

## Main Types

- `TextCodec`
- `JsonTextCodec`
- `CompactJsonTextCodec`
- `CodecError`

## Basic Usage

```rust
use pulith_serde_backend::{JsonTextCodec, TextCodec};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Example {
    schema_version: u32,
}

let codec = JsonTextCodec;
let encoded = codec.encode_pretty(&Example { schema_version: 1 })?;
let decoded: Example = codec.decode_str(&encoded)?;
assert_eq!(decoded.schema_version, 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Helpers

- `encode_pretty_vec(...)`
- `decode_slice(...)`

These helpers are used by state/store/install persistence boundaries to avoid repeating format glue.

## See Also

- `docs/design/serialization.md`
