# pulith-shim Design

## Overview

Platform-independent executable shim mechanism. Provides the minimal mechanism for dispatching command invocations to target binaries. **Shims are mechanisms, not policy** — all version detection and path resolution policy is delegated to user-provided implementations.

## Scope

**Included:**
- `TargetResolver` trait — the single contract between shim and user policy
- Composable resolvers (`PairResolver`, `TripleResolver`) for fallback/chain patterns
- Template shim binary that resolves and executes targets

**Excluded:**
- Version detection (user implements in their `TargetResolver`)
- Path layout enforcement (user defines in their `TargetResolver`)
- Platform-specific script generation (only Rust binary template)
- Shim management (create/remove/list — user implements in their tool)

## Core Concepts

### TargetResolver

The sole trait defining shim behavior:

```rust
pub trait TargetResolver {
    fn resolve(&self, command: &str) -> Option<PathBuf>;
}
```

The shim binary:
1. Receives command name via `argv[0]` or configuration
2. Calls `resolver.resolve(command_name)`
3. If Some(path), `execve(path, args, env)`
4. If None, prints error and exits non-zero

### Composable Resolvers

```rust
pub struct PairResolver<R1, R2> {
    primary: R1,
    fallback: R2,
}

pub struct TripleResolver<R1, R2, R3> {
    first: R1,
    second: R2,
    third: R3,
}
```

**PairResolver**: Try primary, fall back to secondary if None.

**TripleResolver**: Chain three resolvers in sequence.

## Public API

### TargetResolver Trait

```rust
pub trait TargetResolver {
    /// Resolve a command name to its absolute target path.
    ///
    /// Returns `None` if the command cannot be resolved.
    fn resolve(&self, command: &str) -> Option<PathBuf>;
}
```

### PairResolver

```rust
impl<R1, R2> TargetResolver for PairResolver<R1, R2>
where
    R1: TargetResolver,
    R2: TargetResolver,
{
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        self.primary.resolve(command).or_else(|| self.fallback.resolve(command))
    }
}
```

### TripleResolver

```rust
impl<R1, R2, R3> TargetResolver for TripleResolver<R1, R2, R3>
where
    R1: TargetResolver,
    R2: TargetResolver,
    R3: TargetResolver,
{
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        self.first
            .resolve(command)
            .or_else(|| self.second.resolve(command))
            .or_else(|| self.third.resolve(command))
    }
}
```

## Error Types

```rust
#[derive(Debug, Error)]
pub enum Error {
    #[error("resolution failed for command '{0}': target not found")]
    NotFound(String),

    #[error("resolution failed for command '{0}': {1}")]
    ResolveFailed(String, String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

## Shim Binary Template

The `pulith-shim-bin` crate provides a template binary that:
1. Accepts a `TargetResolver` implementation via generic type parameter
2. Reads command name from `argv[0]`
3. Resolves and executes the target

```rust
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let shim_name = args[0].clone();

    // User's resolver determines target
    let resolver = MyResolver::new();
    let target = resolver.resolve(&shim_name);

    match target {
        Some(path) => {
            let status = Command::new(&path)
                .args(&args[1..])
                .status()
                .expect("failed to execute");
            std::process::exit(status.code().unwrap_or(1));
        }
        None => {
            eprintln!("Error: command not found: {}", shim_name);
            std::process::exit(127);
        }
    }
}
```

## Example Implementations

### Simple Path Resolver

```rust
struct SimpleResolver {
    bin_dir: PathBuf,
}

impl TargetResolver for SimpleResolver {
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        let target = self.bin_dir.join(command);
        if target.exists() {
            Some(target)
        } else {
            None
        }
    }
}
```

### Version-Aware Resolver

```rust
struct VersionAwareResolver {
    versions_dir: PathBuf,
    version_var: String,
}

impl TargetResolver for VersionAwareResolver {
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        let version = std::env::var(&self.version_var).ok()?;
        let target = self.versions_dir
            .join(&version)
            .join("bin")
            .join(command);
        if target.exists() {
            Some(target)
        } else {
            None
        }
    }
}
```

### Fallback Resolver

```rust
use pulith_shim::{PairResolver, TargetResolver};

let resolver = PairResolver::new(
    VersionAwareResolver {
        versions_dir: PathBuf::from("/opt/mytool/versions"),
        version_var: "MYTOOL_VERSION".to_string(),
    },
    SimpleResolver {
        bin_dir: PathBuf::from("/usr/local/bin"),
    },
);
```

## Module Structure

```
pulith-shim/src/
├── lib.rs              # Public exports: TargetResolver, PairResolver, TripleResolver
├── resolver.rs         # Trait and resolver implementations
└── error.rs            # Error type
```

## Dependencies

```toml
[package]
name = "pulith-shim"
version = "0.1.0"
edition = "2024"

[dependencies]
thiserror = { workspace = true }
```

```toml
[package]
name = "pulith-shim-bin"
version = "0.1.0"
edition = "2021"

[dependencies]
pulith-shim = { path = "../pulith-shim" }
```

## Design Decisions

### Why Minimal Design?

1. **Separation of concerns**: Shim mechanism is separate from resolution policy
2. **No built-in version detection**: Different tools have different conventions
3. **No platform scripts**: Only Rust binary, works everywhere Rust compiles
4. **Compile-time resolution**: Generic type parameter, no runtime trait dispatch overhead

### Why Not Trait Object?

```rust
// Could use trait object:
fn resolve(&self, command: &str) -> Option<PathBuf>

// But generic is faster:
fn run<R: TargetResolver>() {
    let resolver = R::new();
    // ...
}
```

Generic type parameter enables:
- Zero-cost abstraction (monomorphization)
- Inline resolver logic
- No heap allocation

### Composable Resolvers

Fallback and chain patterns are common but non-trivial to implement correctly. Providing `PairResolver` and `TripleResolver` gives users tested, efficient compositions without requiring custom trait implementations.

## Composition Patterns

### Fallback

Try primary, then fallback:

```rust
let resolver = PairResolver::new(
    VersionResolver::new(...),
    DefaultResolver::new(...),
);
```

### Chain

Try multiple sources in order:

```rust
let resolver = TripleResolver::new(
    EnvResolver::new(...),
    FileResolver::new(...),
    DefaultResolver::new(...),
);
```

### Custom

Users can implement `TargetResolver` directly for any policy:

```rust
struct CustomResolver { /* ... */ }

impl TargetResolver for CustomResolver {
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        // Custom logic here
    }
}
```
