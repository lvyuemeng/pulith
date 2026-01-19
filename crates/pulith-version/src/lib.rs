//! Version parsing, comparison, and display for multiple version schemes.
//!
//! Supports SemVer, CalVer, and partial versions.

#![recursion_limit = "256"]

pub mod calver;
pub mod partial;
pub mod semver;

use calver::CalVer;
use partial::Partial;
use semver::SemVer;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VersionKind {
    SemVer(SemVer),
    CalVer(CalVer),
    Partial(Partial),
}

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("invalid semver")]
    SemVer,
    #[error(transparent)]
    CalVer(#[from] calver::CalVerError),
    #[error(transparent)]
    Partial(#[from] partial::PartialError),
    #[error("unknown version scheme")]
    Unknown,
}

impl std::str::FromStr for VersionKind {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try SemVer first (most specific format)
        if let Ok(v) = s.parse::<SemVer>() {
            return Ok(VersionKind::SemVer(v));
        }
        // Try CalVer (requires 4-digit year)
        if let Ok(v) = s.parse::<CalVer>() {
            return Ok(VersionKind::CalVer(v));
        }
        // Fall back to partial
        match s.parse::<Partial>() {
            Ok(p) => Ok(VersionKind::Partial(p)),
            Err(e) => Err(VersionError::Partial(e)),
        }
    }
}

impl std::fmt::Display for VersionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionKind::SemVer(v) => write!(f, "{}", v),
            VersionKind::CalVer(v) => write!(f, "{}", v),
            VersionKind::Partial(v) => write!(f, "{}", v),
        }
    }
}

impl VersionKind {
    /// Parse string into VersionKind.
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        s.parse()
    }

    /// Convert to SemVer if possible.
    pub fn as_semver(&self) -> Option<&SemVer> {
        match self {
            VersionKind::SemVer(v) => Some(v),
            _ => None,
        }
    }

    /// Get version kind type.
    pub fn kind(&self) -> VersionKindType {
        match self {
            VersionKind::SemVer(_) => VersionKindType::SemVer,
            VersionKind::CalVer(_) => VersionKindType::CalVer,
            VersionKind::Partial(_) => VersionKindType::Partial,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionKindType {
    SemVer,
    CalVer,
    Partial,
}

#[cfg(test)]
mod tests {
    use super::{VersionKind, VersionKindType};

    #[test]
    fn test_semver_parse() {
        let v: VersionKind = "1.2.3".parse().unwrap();
        assert_eq!(v.kind(), VersionKindType::SemVer);
    }

    #[test]
    fn test_calver_parse() {
        let v: VersionKind = "2024.01".parse().unwrap();
        assert_eq!(v.kind(), VersionKindType::CalVer);
    }

    #[test]
    fn test_partial_major() {
        let v: VersionKind = "18".parse().unwrap();
        assert_eq!(v.kind(), VersionKindType::Partial);
    }

    #[test]
    fn test_version_comparison() {
        let v1: VersionKind = "1.0.0".parse().unwrap();
        let v2: VersionKind = "2.0.0".parse().unwrap();
        assert!(v1 < v2);
    }

    #[test]
    fn test_version_display() {
        let v: VersionKind = "1.2.3".parse().unwrap();
        assert_eq!(format!("{}", v), "1.2.3");
    }
}
