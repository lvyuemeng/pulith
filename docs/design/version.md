# pulith-version Design

## Philosophy Alignment

This crate follows the F1-F5 philosophy with a **pure core** and **no I/O**:

| Principle | Alignment | Implementation |
|-----------|-----------|----------------|
| **F1: Functions First** | ✓ | All operations are pure functions (`parse`, `matches`, comparison) |
| **F2: Immutability** | ✓ | All types use `Clone` + derived traits, no mutability |
| **F3: Pure Core** | ✓ | No I/O; version parsing is purely functional |
| **F4: Explicit Effects** | ✓ | Effects layer is empty (no side effects possible) |
| **F5: Composition** | ✓ | `VersionKind` composes `SemVer`, `CalVer`, `Partial` |

### Three-Layer Pattern

```
data.rs     → Immutable types (VersionKind, CalVer, Partial, errors)
core.rs     → Pure operations (FromStr, Display, parsing logic)
effects.rs  → Empty (all operations are pure)
```

## Overview

Version parsing, comparison, and display for multiple version schemes. Supports SemVer, CalVer, and partial versions.

## Scope

**Included:**
- SemVer (Semantic Versioning 2.0)
- CalVer (Calendar Versioning)
- Partial versions (major-only, major.minor, custom patterns)
- Version comparison and ordering

**Excluded:**
- Version constraints/specs (for future crate)
- Version fetching from remote (handled by fetch crate)
- Version storage (handled by registry crate)

## Public API

```rust
/// Version kind enum
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VersionKind {
    SemVer(semver::Version),
    CalVer(CalVer),
    Partial(Partial),
}

/// Calendar Versioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalVer(semver::Version);

/// Partial version (major.minor, major only, etc.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Partial {
    pub major: Option<u64>,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
    pub lts: bool,
}

/// Parsing error
#[derive(Debug, Error)]
pub enum VersionError {
    #[error("invalid semver")]
    SemVer(#[from] semver::Error),

    #[error(transparent)]
    CalVer(#[from] calver::CalVerError),

    #[error(transparent)]
    Partial(#[from] partial::PartialError),

    #[error("unknown version scheme")]
    Unknown,
}

// Core traits
impl VersionKind {
    /// Parse string into VersionKind
    pub fn parse(s: &str) -> Result<Self, VersionError>;

    /// Convert to SemVer if possible
    pub fn as_semver(&self) -> Option<&semver::Version>;

    /// Get version kind type
    pub fn kind(&self) -> VersionKindType;
}

impl CalVer {
    /// Parse CalVer string (YYYY, YYYY.MM, YYYY.MM.DD, etc.)
    pub fn parse(s: &str) -> Result<Self, CalVerError>;

    /// Create from date components
    pub fn from_ymd(year: u64, month: u64, day: u64) -> Result<Self, CalVerError>;
}

impl Partial {
    /// Parse partial version string
    pub fn parse(s: &str) -> Result<Self, PartialError>;

    /// Check if this partial matches a full version
    pub fn matches(&self, version: &VersionKind) -> bool;
}

/// Version kind type indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionKindType {
    SemVer,
    CalVer,
    Partial,
}
```

## Module Structure

```
pulith-version/src/
├── lib.rs              # Public API re-exports and crate docs
└── version.rs          # All version types and operations
```

## Dependencies

```toml
[dependencies]
semver    # Semantic versioning
regex     # Partial version parsing
thiserror # Error handling
```

> Exact versions in `crates/pulith-version/Cargo.toml` for timeliness.

## Examples

### Parse and Compare Versions

```rust
use pulith_version::VersionKind;

let v1: VersionKind = "1.2.3".parse().unwrap();
let v2: VersionKind = "2.0.0".parse().unwrap();

assert!(v1 < v2);
```

### SemVer with Pre-release

```rust
use pulith_version::SemVer;

let v1 = SemVer::new(1, 0, 0);
let v2 = "1.0.0-alpha".parse::<SemVer>().unwrap();

assert!(v2 < v1);
```

### Calendar Versioning

```rust
use pulith_version::CalVer;

let v2024 = "2024".parse::<CalVer>().unwrap();
let v202401 = "2024.01".parse::<CalVer>().unwrap();

assert!(v2024 < v202401);
```

### Partial Versions

```rust
use pulith_version::Partial;

let major18 = "18".parse::<Partial>().unwrap();
let major3_minor11 = "3.11".parse::<Partial>().unwrap();

// Partial versions sort by available components
assert!(major18 < major3_minor11);

// Check if full version matches partial
let v18_0_0: VersionKind = "18.0.0".parse().unwrap();
assert!(major18.matches(&v18_0_0));
```

## Version Formats

### SemVer
```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]
1.2.3
1.2.3-alpha
1.2.3+build.123
1.2.3-alpha+build.123
```

### CalVer
```
YYYY
YYYY.MM
YYYY.MM.DD
2024
2024.01
2024.01.15
```

### Partial
```
major
major.minor
major.minor.patch
major.lts
18
3.11
3.11.0
18lts
```

## Design Decisions

### Why Use semver::Version Directly?

- Battle-tested parsing and comparison
- No need to wrap when we want standard SemVer behavior
- Consistent with CalVer which uses semver::Version internally

### CalVer Implementation

- Uses semver::Version internally (YYYY.MM.DD maps to major.minor.patch)
- Handles various CalVer formats (YYYY, YYYY.MM, YYYY.MM.DD)
- Sorts correctly via semver ordering

### Partial Version Design

- Flexible matching (18 matches 18.0.0, 18.5.2, etc.)
- Used by version managers (nvm "use 18", pyenv "use 3.11")
- Component-wise comparison for ordering

### No Constraint/Spec Support

- Constraints (^1.0.0, >=2.0) are complex
- Better as separate crate when needed
- Keep this crate focused on parsing/comparison

## Comparison Behavior

| Version 1 | Version 2 | Result |
|-----------|-----------|--------|
| 1.0.0 | 1.0.1 | v1 < v2 |
| 1.0.0 | 2.0.0 | v1 < v2 |
| 1.0.0-alpha | 1.0.0 | v1 < v2 |
| 2024 | 2024.01 | v1 < v2 |
| 3.11 | 3.12 | v1 < v2 |
| 18 | 18.0.0 | v1 == v2 |

Note: Partial versions compare based on available components.
