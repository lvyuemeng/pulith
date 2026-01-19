# Pulith Coding Specifications

## Language and Tooling

- **Rust Edition**: 2024
- **Minimum Rust Version**: 1.75.0
- **Formatting**: `cargo fmt` with default configuration
- **Linting**: `cargo clippy` with strict warnings
- **Documentation**: `cargo doc` for API documentation

## Workspace Structure

```
pulith/
├── Cargo.toml           # Workspace manifest
├── crates/
│   ├── pulith-*/       # Individual crates
│   └── ...
└── Cargo.lock          # Lock file (committed)
```

## Dependencies

- **Workspace Dependencies**: Declare in root `Cargo.toml` under `[workspace.dependencies]`
- **Crate Dependencies**: Reference workspace deps or add crate-specific ones
- **Feature Gates**: Gate heavy dependencies (HTTP, archive formats, crypto) behind features
- **Version Pinning**: Use `=` for critical dependencies, `*` for flexible matching

## Error Handling

### Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("contextual message")]
    Variant { field: Type },

    #[error(transparent)]
    External(#[from] external::Error),
}
```

### Propagation

- **Application code**: Use `anyhow` with `?` operator
- **Library code**: Return concrete error types
- **Never** use `unwrap()`, `expect()`, or `panic!()` in library code

## Async and Concurrency

- **Runtime**: Use shared `tokio` runtime from `pulith_core::task_pool`
- **Pattern**: `POOL.block_on(async { ... })` for blocking async calls
- **Never** spawn new runtimes in library code
- **Thread Safety**: All public types must be `Send + Sync`

## Cross-Platform Requirements

### Path Handling

- Use `PathBuf`, never `String` for paths
- Use `std::path::Path` for parameters
- Handle path separators (Windows backslash, Unix forward slash)

### OS-Specific Code

```rust
#[cfg(target_os = "windows")]
fn windows_specific() { ... }

#[cfg(target_os = "linux")]
fn linux_specific() { ... }

#[cfg(not(windows))]
fn non_windows() { ... }
```

### Platform Detection

Use `pulith_platform` for OS and distribution detection.

## Serialization

- **Format**: Binary with `postcard`, JSON with `serde_json`
- **Derive**: Use `#[derive(Serialize, Deserialize)]`
- **Skipping**: Use `#[serde(skip)]` for non-serialized fields
- **Visibility**: Keep serialized representation as implementation detail

## Code Organization

### Module Structure

```
crate/
├── src/
│   ├── lib.rs          # Public API re-exports
│   ├── prelude.rs      # Common imports (optional)
│   ├── error.rs        # Error types (optional)
│   └── ...
└── tests/
    └── integration.rs  # Integration tests
```

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Crates | `pulith-*` | `pulith-version` |
| Traits | PascalCase | `Source`, `Tracker` |
| Errors | `Error` suffix | `VersionError` |
| Builders | `*Builder` | `ProgressTrackerBuilder` |
| Modules | snake_case | `reg`, `ui` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRY_COUNT` |

### Visibility

- **Public API**: Mark with `pub` at crate root
- **Internal**: Use `pub(crate)` for intra-crate public
- **Private**: Omit `pub` for module-private items

## Testing

### Requirements

- **Unit Tests**: In `#[cfg(test)]` modules alongside code
- **Integration Tests**: In `tests/` directory
- **Property Tests**: For parsing and comparison logic
- **Coverage**: Aim for 80%+ coverage on critical paths

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() { ... }

    #[test]
    fn test_version_comparison() { ... }
}
```

## Documentation

### Public API

All public items must have `///` documentation:

```rust
/// Parses a version string into a [`Version`].
///
/// # Errors
///
/// Returns [`VersionError`] if the string is not a valid version.
///
/// # Examples
///
/// ```
/// use pulith_version::Version;
///
/// let version = "1.2.3".parse().unwrap();
/// assert_eq!(version.major, 1);
/// ```
pub fn parse_version(s: &str) -> Result<Version, VersionError> { ... }
```

### Error Documentation

Document error variants with `#[error(...)]`:

```rust
#[derive(Error, Debug)]
pub enum VersionError {
    #[error("invalid version format: {0}")]
    InvalidFormat(String),

    #[error("unknown version scheme: {0}")]
    UnknownScheme(String),
}
```

## Code Style

### Imports

```rust
// Standard library
use std::path::{Path, PathBuf};

// Third party
use anyhow::{Context, Result};
use thiserror::Error;

// Crate local
use crate::module::Item;
```

### Formatting

- Run `cargo fmt` before committing
- Maximum line width: 100 characters
- Use Rust-analyzer or similar IDE support

### Anti-Patterns to Avoid

- `unwrap()`, `expect()`, `panic!()` in library code
- Spawning new async runtimes
- Using `String` instead of `PathBuf` for paths
- `#[allow(dead_code)]` without explanation
- Magic numbers (use constants)

## Pull Requests

1. **Branch**: `feature/<short-description>`
2. **Size**: Keep changes small and focused
3. **Tests**: Include tests for new functionality
4. **Docs**: Update documentation for API changes
5. **CI**: Ensure all checks pass

## References

- [design.md](./design.md) - Architecture and crate design
- [README.md](./README.md) - Project overview
