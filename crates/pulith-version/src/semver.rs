//! Semantic Versioning wrapper.

use crate::VersionError;
use semver::Version;
use std::ops::Deref;

/// Semantic Versioning wrapper.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemVer(Version);

impl SemVer {
    /// Create new SemVer.
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self(Version::new(major, minor, patch))
    }

    /// Access underlying semver crate Version.
    pub fn inner(&self) -> &Version {
        &self.0
    }

    /// Get major version.
    pub fn major(&self) -> u64 {
        self.0.major
    }

    /// Get minor version.
    pub fn minor(&self) -> u64 {
        self.0.minor
    }

    /// Get patch version.
    pub fn patch(&self) -> u64 {
        self.0.patch
    }

    /// Get pre-release identifier.
    pub fn pre(&self) -> &str {
        &self.0.pre
    }

    /// Get build metadata.
    pub fn build(&self) -> &str {
        &self.0.build
    }
}

impl std::str::FromStr for SemVer {
    type Err = crate::VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse().map_err(|_| VersionError::Unknown)?))
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for SemVer {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
