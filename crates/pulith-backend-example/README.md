# pulith-backend-example

Thin adapter-first backend example built on Pulith crates.

## Main APIs

- `ManagedBinarySpec`
- `managed_binary(...)`

## Basic Usage

```rust
use pulith_backend_example::managed_binary;
use pulith_resource::{ResourceLocator, ValidUrl, VersionSelector};

let spec = managed_binary(
    "example/runtime",
    ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip")?),
    VersionSelector::alias("stable")?,
    "/installs/runtime",
    "bin/runtime",
)?;
let _requested = spec.requested_resource();
# Ok::<(), Box<dyn std::error::Error>>(())
```

See `docs/design.md` and `examples/runtime-manager/`.
