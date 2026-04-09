# pulith-backend-example

Thin adapter-first backend example built on Pulith crates.

## Role

This crate demonstrates how to shape a backend-facing API without hiding fetch/store/state/install policy inside a framework.

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

## How To Use It

Treat this crate as an adapter pattern reference, not a framework. It shows how to shape public backend inputs while leaving composition explicit.

See `docs/design.md` and `examples/runtime-manager/`.
