# Pulith Architecture

## Philosophy

Pulith is built on five foundational principles that guide all design decisions.

- Mechanism-only: Provide primitives to fetch, store, stage, and track external resources.

- Concrete-independent: Don’t decide concrete implementation or design— only expose APIs that let higher layers handle that.

- Atomic and safe: All operations should be rollback-safe and cross-platform.

- Composable: Install, registry, and shim layers can be combined in any order, but each is independent.

- Minimal contract: Never assume a specific JSON schema, version format, or registry format.

### F1: Functions First

Behavior is expressed as `output = f(input)`. Avoid hidden state, magic side effects, and behaviorful objects.

**Anti-pattern:**
```rust
pub struct Manager {
    state: Mutex<State>,
    config: Config,
}

impl Manager {
    pub fn process(&self, input: Input) -> Result<Output> {
        // Hidden state mutation
        // Implicit I/O
        // Non-obvious control flow
    }
}
```

**Preferred:**
```rust
pub struct Config { /* ... */ }

pub fn process(input: Input, config: &Config) -> Result<Output> {
    // Pure transformation
    // Explicit dependencies
    // Composable
}
```

### F2: Immutability by Default

Core data is immutable; mutation is allowed only at system boundaries (I/O, caches, buffers).

**Anti-pattern:**
```rust
pub struct Builder {
    field: Type,
    another_field: Type,
}

impl Builder {
    pub fn set_field(&mut self, value: Type) -> &mut Self {
        self.field = value;
        self
    }
}
```

**Preferred:**
```rust
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub field: Type,
    pub another_field: Type,
}

impl Config {
    pub fn with_field(mut self, value: Type) -> Self {
        self.field = value;
        self
    }
}
```

### F3: Pure Core, Impure Edge

All reasoning lives in a pure core. All effects (model calls, tools, storage, logging) live at the edge.

```
+------------------+     +------------------+     +------------------+
|     Data Layer   | --> |    Core Layer    | --> |   Effect Layer   |
|   (Immutable)    |     |   (Deterministic)|     |   (I/O, Effects) |
+------------------+     +------------------+     +------------------+
| Config, Options, |     | Transform,       |     | File operations, |
| State, Query     |     | Validate,        |     | Network calls,   |
|                  |     | Compute,         |     | Environment      |
+------------------+     +------------------+     +------------------+
```

### F4: Explicit Effects

If a function has effects or uses randomness, this must be explicit in its interface.

**Anti-pattern:**
```rust
pub fn detect_version() -> Result<Option<String>> {
    // Reads environment
    // Reads files
    // Unclear from signature
}
```

**Preferred:**
```rust
pub trait FileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, io::Error>;
}

pub fn detect_version<F: FileSystem>(
    source: &VersionSource,
    env: &Environment,
    fs: &F,
) -> Result<Option<String>> {
    // Effect dependency is explicit
}
```

### F5: Composition Over Orchestration

Prefer composable pipelines over ad-hoc control flow or global state.

**Anti-pattern:**
```rust
pub fn run_pipeline(input: &str) -> Result<Output> {
    let a = step_a(input)?;
    let b = step_b(&a)?;
    let c = step_c(&b)?;
    // Hard to test, hard to reuse
    step_d(&c)
}
```

**Preferred:**
```rust
pub fn pipeline(input: Input) -> impl Pipeline {
    Pipeline::new(input)
        .then(step_a)
        .then(step_b)
        .then(step_c)
        .then(step_d)
}
```

## Three-Layer Architecture

Every crate follows a consistent three-layer structure.

### 1. Data Layer

Immutable types that model the problem domain. No I/O, no side effects.

```rust
// data.rs

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub root: PathBuf,
    pub max_retries: u32,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub verbose: bool,
    pub dry_run: bool,
}

pub struct Query {
    pub pattern: String,
    pub filters: Vec<Filter>,
}
```

### 2. Core Layer

Pure, deterministic transformations over data. No hidden state, no I/O.

```rust
// core.rs

pub fn validate(config: &Config) -> Result<(), ValidationError> {
    if config.max_retries > 100 {
        return Err(ValidationError::MaxRetriesExceeded);
    }
    Ok(())
}

pub fn transform(input: Input, config: &Config) -> Output {
    // Pure computation
    // No side effects
    // Referentially transparent
}

pub fn detect_version(source: &VersionSource, env: &Environment) -> Option<String> {
    // Deterministic logic
    // No file I/O
}
```

### 3. Effect Layer

All I/O operations. Receives abstractors (traits) for testability.

```rust
// effects.rs

pub trait FileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, io::Error>;
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), io::Error>;
    fn create_dir_all(&self, path: &Path) -> Result<(), io::Error>;
}

pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, io::Error> {
        std::fs::read_to_string(path)
    }
    // ... other methods
}

pub fn load_config<F: FileSystem>(
    path: &Path,
    fs: &F,
) -> Result<Config, LoadError> {
    let content = fs.read_to_string(path)?;
    // Parse and return
}

pub fn atomic_replace<F: FileSystem>(
    src: &Path,
    dst: &Path,
    fs: &F,
) -> Result<(), AtomicReplaceError> {
    // File operations with abstracted FS
}
```

## Effect Abstraction Pattern

### Minimal Trait Design

Each crate defines a minimal `FileSystem` trait specific to its needs, not a general-purpose one.

**For pulith-fetch:**
```rust
trait DownloadFileSystem {
    fn create(&self, path: &Path) -> Result<File, io::Error>;
    fn write_all(&self, file: &mut File, bytes: &[u8]) -> Result<(), io::Error>;
    fn set_permissions(&self, path: &Path, perms: Permissions) -> Result<(), io::Error>;
}
```

**For pulith-shim:**
```rust
trait ShimFileSystem {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), io::Error>;
    fn remove_file(&self, path: &Path) -> Result<(), io::Error>;
    fn set_permissions(&self, path: &Path, perms: Permissions) -> Result<(), io::Error>;
    #[cfg(unix)]
    fn symlink(&self, original: &Path, link: &Path) -> Result<(), io::Error>;
}
```

**For pulith-core:**
```rust
trait InstallFileSystem {
    fn create_dir_all(&self, path: &Path) -> Result<(), io::Error>;
    fn rename(&self, from: &Path, to: &Path) -> Result<(), io::Error>;
    fn copy(&self, from: &Path, to: &Path) -> Result<(), io::Error>;
    fn remove_file(&self, path: &Path) -> Result<(), io::Error>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), io::Error>;
    #[cfg(unix)]
    fn symlink(&self, original: &Path, link: &Path) -> Result<(), io::Error>;
}
```

### Test Implementations

```rust
#[cfg(test)]
pub struct MemFileSystem {
    files: RefCell<HashMap<PathBuf, Vec<u8>>>,
}

impl DownloadFileSystem for MemFileSystem {
    fn create(&self, path: &Path) -> Result<File, io::Error> {
        // In-memory file implementation
    }
}
```

## Module Layout Guidelines

The three-layer pattern (Data/Core/Effects) is a **guideline**, not a mandate. Apply it pragmatically based on crate complexity.

### When to Apply Three-Layer Pattern

| Crate Type | I/O | Size | Structure |
|------------|-----|------|-----------|
| **I/O-heavy** (fetch, install) | Yes | Any | Full three-layer (data/core/effects) |
| **Pure with effects** (shim) | FileSystem | Medium | Full three-layer |
| **Pure parsing** (version) | None | <500 lines | Single file or 2 files (lib + domain) |
| **Utilities** (platform) | Env | <500 lines | Single file or 2 files (lib + domain) |

### Pragmatic Guidelines

1. **<500 lines total**: Single file (`lib.rs`)
2. **500-1000 lines**: Two files (`lib.rs` + `domain.rs`)
3. **>1000 lines or complex I/O**: Full three-layer (`data.rs`, `core.rs`, `effects.rs`)
4. **Error types**: Keep with related functionality unless shared widely
5. **Tests**: Inline in the module they test

### Recommended Structures

**Single file (pure crate):**
```
pulith-version/src/
└── lib.rs              # Everything (types, impls, tests)
```

**Two files (medium crate):**
```
pulith-platform/src/
├── lib.rs              # Docs + re-exports
└── platform.rs         # All platform utilities
```

**Three-layer (complex I/O crate):**
```
pulith-shim/src/
├── lib.rs              # Docs + re-exports
├── data.rs             # Immutable types
├── core.rs             # Pure operations
└── effects.rs          # I/O with trait bounds
```

### lib.rs Pattern (Minimal)

```rust
//! Crate documentation

pub use self::{DomainType, Function, Error};

mod domain;
```

### Three-layer lib.rs Pattern

```rust
//! Crate documentation
//!
//! This crate follows the three-layer pattern:
//! - [`data`] - Immutable configuration and types
//! - [`core`] - Pure transformations
//! - [`effects`] - I/O operations with trait abstraction

pub use data::{Config, Options};
pub use core::{process, transform};
pub use effects::{load, save};

mod data;
mod core;
mod effects;
mod error;
pub use error::{Error, Result};
```

### Import Convention

```rust
// prelude.rs (optional, for internal use)
pub use std::path::{Path, PathBuf};

pub use crate::data::{Config, Options};
pub use crate::error::{Error, Result};

// In modules
use crate::data::Config;
```

## Composition Patterns

### Pipeline with `and_then`

```rust
pub fn pipeline(input: Input) -> Result<Output> {
    input
        .and_then(validate)
        .and_then(transform)
        .and_then(save)
}

fn validate(input: Input) -> Result<ValidInput> { /* ... */ }
fn transform(input: ValidInput) -> Result<Transformed> { /* ... */ }
fn save(input: Transformed) -> Result<Output> { /* ... */ }
```

### Builder for Effect Configuration

Use builders only for configuring effect layers, not for hiding state:

```rust
pub struct DownloadBuilder<F: FileSystem> {
    fs: F,
    options: DownloadOptions,
    retries: u32,
}

impl<F: FileSystem> DownloadBuilder<F> {
    pub fn new(fs: F) -> Self {
        Self { fs, options: DownloadOptions::default(), retries: 3 }
    }

    pub fn with_options(mut self, options: DownloadOptions) -> Self {
        self.options = options;
        self
    }

    pub fn retries(mut self, n: u32) -> Self {
        self.retries = n;
        self
    }

    pub fn download(self, url: &Url) -> Result<PathBuf> {
        effects::download(url, &self.options, self.retries, &self.fs)
    }
}
```

## Error Handling

### Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("contextual message")]
    Variant { field: Type },

    #[error(transparent)]
    External(#[from] external::Error),
}
```

### Error Propagation

- **Library code**: Return concrete error types
- **Application code**: Use `anyhow` with `?` operator
- **Never** use `unwrap()`, `expect()`, or `panic!()` in library code

## Cross-Platform Considerations

### Path Handling

- Use `PathBuf` for owned paths, `&Path` for references
- Never use `String` for paths
- Handle path separators (Windows `\;`, Unix `/`)

### OS-Specific Code

```rust
#[cfg(target_os = "windows")]
pub fn platform_specific() { /* Windows implementation */ }

#[cfg(target_os = "linux")]
pub fn platform_specific() { /* Linux implementation */ }

#[cfg(not(windows))]
pub fn non_windows() { /* Unix-only */ }
```

### Platform Detection

Use `pulith_platform` for OS and distribution detection:

```rust
use pulith_platform::os::{detect, OS};

let os = detect();
match os {
    OS::Windows => { /* Windows path */ }
    OS::Macos => { /* macOS path */ }
    OS::Linux(distro) => { /* Linux path */ }
    OS::Unknown => { /* Fallback */ }
}
```

## References

- [README.md](../README.md) - Project overview
- [AGENT.md](../AGENT.md) - Coding specifications
- [design.md](./design.md) - Architecture and crate design
- [design/*.md](./design/) - Individual crate designs
