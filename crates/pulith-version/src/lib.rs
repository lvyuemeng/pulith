//! Version parsing, comparison, and display for multiple version schemes.
//!
//! Supports SemVer, CalVer, and partial versions.
//!
//! # Version Schemes
//!
//! - **SemVer**: Semantic Versioning 2.0 (`1.2.3`, `1.0.0-alpha`)
//! - **CalVer**: Calendar Versioning (`2024`, `2024.01.15`)
//! - **Partial**: Partial versions for matching (`18`, `3.11`)

pub use self::version::{CalVer, Partial, VersionError, VersionKind, VersionKindType};

mod version;
