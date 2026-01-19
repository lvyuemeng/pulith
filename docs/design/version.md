# pulith-version Design

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
    SemVer(SemVer),
    CalVer(CalVer),
    Partial(Partial),
}

/// Semantic Versioning wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemVer(semver::Version);

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
    #[error("invalid SemVer: {0}")]
    SemVer(#[from] semver::Error),

    #[error("invalid CalVer format: {0}")]
    CalVer(String),

    #[error("invalid partial version: {0}")]
    Partial(String),

    #[error("unknown version scheme")]
    Unknown,
}

// Core traits
impl VersionKind {
    /// Parse string into VersionKind
    pub fn parse(s: &str) -> Result<Self, VersionError>;

    /// Get version as display string
    pub fn to_string(&self) -> String;

    /// Convert to SemVer if possible
    pub fn as_semver(&self) -> Option<SemVer>;

    /// Get version kind type
    pub fn kind(&self) -> VersionKindType;
}

impl SemVer {
    /// Create new SemVer
    pub fn new(major: u64, minor: u64, patch: u64) -> Self;

    /// Access underlying semver crate Version
    pub fn inner(&self) -> &semver::Version;
}

impl CalVer {
    /// Parse CalVer string (YYYY, YYYY.MM, YYYY.MM.DD, etc.)
    pub fn parse(s: &str) -> Result<Self, VersionError>;

    /// Create from date components
    pub fn from_ymd(year: u64, month: u64, day: u64) -> Result<Self, VersionError>;
}

impl Partial {
    /// Parse partial version string
    pub fn parse(s: &str) -> Result<Self, VersionError>;

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
├── lib.rs              # Public exports and main types
├── semver.rs           # SemVer wrapper
├── calver.rs           # Calendar versioning
└── partial.rs          # Partial version parsing
```

## Dependencies

```toml
[dependencies]
semver    # Semantic versioning
regex     # Partial version parsing
serde     # Serialization support
thiserror # Error handling
```

> Exact versions in `crates/pulith-version/Cargo.toml` for timeliness.

## Examples

### Parse and Compare Versions

```rust
use pulith_version::{VersionKind, VersionError};

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

### Why Wrap semver Crate?

- Use battle-tested semver parsing
- Add custom version kinds alongside SemVer
- Consistent API across all version types

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
