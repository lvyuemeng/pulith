# pulith-shim-bin

Thin shim-binary helper/template.

## Role

`pulith-shim-bin` bridges `pulith-shim` into a runnable executable boundary.

It is a support/template crate, not a semantic or workflow crate.

## Main APIs

- `try_run(...)`
- `invoke(...)`
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

## How To Use It

Copy or adapt this crate when you need a tiny executable that:

- resolves a command target through `pulith-shim`
- validates the resolved path
- forwards execution

See `docs/design/shim.md`.
