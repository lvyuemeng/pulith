# pulith-shim-bin

Template shim binary support for Pulith.

## Purpose

Use this crate when you want a small executable that resolves a command target through `pulith-shim` and forwards execution.

## Main APIs

- `try_run(...)`
- `Error`

## Basic Usage

```rust
use pulith_shim::TargetResolver;
use pulith_shim_bin::try_run;

# struct Resolver;
# impl TargetResolver for Resolver {
#     fn resolve(&self, _command: &str) -> Option<std::path::PathBuf> { None }
# }
let _ = try_run(Resolver);
```

See `docs/design/shim.md`.
