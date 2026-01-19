//! Partial version parsing (major-only, major.minor, etc.).

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

static PARTIAL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?<major>[0-9]+))?(?:\.(?<minor>[0-9]+))?(?:\.(?<patch>[0-9]+))?(?:-(?<pre>[a-zA-Z][-0-9a-zA-Z.]*))?(?:\+(?<build>[-0-9a-zA-Z.]+))?(?:lts)?$").unwrap()
});

/// Partial version error.
#[derive(Debug, Error)]
#[error("invalid partial version: {0}")]
pub struct PartialError(pub String);

/// Partial version (major-only, major.minor, etc.).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Partial {
    pub major: Option<u64>,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
    pub lts: bool,
}

impl Partial {
    /// Parse partial version string.
    pub fn parse(s: &str) -> Result<Self, PartialError> {
        let trimmed = s.trim();
        let lts = trimmed.ends_with("lts");
        let trimmed = trimmed.trim_end_matches("lts");

        let (parts, build) = trimmed
            .split_once('+')
            .map(|(c, b)| (c, Some(b)))
            .unwrap_or((trimmed, None));
        let (parts, pre) = parts
            .split_once('-')
            .map(|(c, p)| (c, Some(p)))
            .unwrap_or((parts, None));

        let caps = PARTIAL_REGEX
            .captures(parts)
            .ok_or_else(|| PartialError(s.to_string()))?;

        let major = caps.name("major").and_then(|m| m.as_str().parse().ok());
        let minor = caps.name("minor").and_then(|m| m.as_str().parse().ok());
        let patch = caps.name("patch").and_then(|m| m.as_str().parse().ok());

        if major.is_none() && minor.is_none() && patch.is_none() {
            return Err(PartialError(s.to_string()));
        }

        Ok(Partial {
            major,
            minor,
            patch,
            pre_release: pre.map(|s| s.to_string()),
            build_metadata: build.map(|s| s.to_string()),
            lts,
        })
    }

    /// Check if this partial matches a full version.
    pub fn matches(&self, version: &super::VersionKind) -> bool {
        match version {
            super::VersionKind::SemVer(v) => {
                self.major.map_or(true, |m| m == v.major())
                    && self.minor.map_or(true, |m| m == v.minor())
                    && self.patch.map_or(true, |m| m == v.patch())
            }
            super::VersionKind::CalVer(v) => {
                self.major.map_or(true, |m| m == v.year())
                    && self.minor.map_or(true, |m| m == v.month())
                    && self.patch.map_or(true, |m| m == v.day())
            }
            super::VersionKind::Partial(other) => {
                self.major == other.major && self.minor == other.minor && self.patch == other.patch
            }
        }
    }
}

impl std::str::FromStr for Partial {
    type Err = PartialError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Partial::parse(s)
    }
}

impl std::fmt::Display for Partial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(major) = self.major {
            write!(f, "{}", major)?;
        }
        if let Some(minor) = self.minor {
            write!(f, ".{}", minor)?;
        }
        if let Some(patch) = self.patch {
            write!(f, ".{}", patch)?;
        }
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build_metadata {
            write!(f, "+{}", build)?;
        }
        if self.lts {
            write!(f, "lts")?;
        }
        Ok(())
    }
}
