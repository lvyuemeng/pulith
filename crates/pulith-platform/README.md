# pulith-platform

Small cross-platform helpers for Pulith.

## Purpose

Use this crate for OS, shell, command, environment, and directory helpers that should stay independent from higher-level resource-management flow.

## Main APIs

- `arch`
- `command`
- `dir`
- `env`
- `os`
- `shell`

## Basic Usage

```rust
use pulith_platform::Result;
```

See `docs/design/platform.md` for the platform boundary.
