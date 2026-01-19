//! Calendar Versioning.

use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version;
use thiserror::Error;

static CALVER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?<year>[0-9]{4})[-.](?<month>(0?[1-9]|10|11|12))(?:\.(?<day>(0?[1-9]|[1-3][0-9])))?(?:\+(?<micro>[0-9]+))?(?:-(?<pre>[a-zA-Z][-0-9a-zA-Z.]+))?$").unwrap()
});

/// Calendar Versioning error.
#[derive(Debug, Error)]
pub enum CalVerError {
    #[error("invalid CalVer format: {0}")]
    InvalidFormat(String),
}

/// Calendar Versioning wrapper.
/// Uses semver internally for comparison.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalVer(Version);

impl std::str::FromStr for CalVer {
    type Err = CalVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CalVer::parse(s)
    }
}

impl CalVer {
    /// Parse CalVer string (YYYY, YYYY.MM, YYYY.MM.DD, etc.).
    pub fn parse(s: &str) -> Result<Self, CalVerError> {
        let caps = CALVER_REGEX
            .captures(s)
            .ok_or_else(|| CalVerError::InvalidFormat(s.to_string()))?;

        let year = caps
            .name("year")
            .map(|c| c.as_str().trim_start_matches('0'))
            .unwrap_or("0");
        let year = if year.len() < 4 {
            format!("20{}", year)
        } else {
            year.to_string()
        };

        let month = caps
            .name("month")
            .map(|c| c.as_str().trim_start_matches('0'))
            .unwrap_or("0");
        let day = caps
            .name("day")
            .map(|c| c.as_str().trim_start_matches('0'))
            .unwrap_or("0");

        let mut version = format!("{}.{}.{}", year, month, day);

        if let Some(pre) = caps.name("pre") {
            version.push('-');
            version.push_str(pre.as_str());
        }

        if let Some(micro) = caps.name("micro") {
            version.push('+');
            version.push_str(micro.as_str());
        }

        Ok(Self(
            Version::parse(&version).map_err(|_| CalVerError::InvalidFormat(s.to_string()))?,
        ))
    }

    /// Create from date components.
    pub fn from_ymd(year: u64, month: u64, day: u64) -> Result<Self, CalVerError> {
        if month < 1 || month > 12 {
            return Err(CalVerError::InvalidFormat(format!(
                "invalid month: {}",
                month
            )));
        }
        if day < 1 || day > 31 {
            return Err(CalVerError::InvalidFormat(format!("invalid day: {}", day)));
        }

        let version = format!("{:04}.{:02}.{:02}", year, month, day);
        Ok(Self(
            Version::parse(&version).map_err(|_| CalVerError::InvalidFormat(version))?,
        ))
    }

    /// Get year.
    pub fn year(&self) -> u64 {
        self.0.major
    }

    /// Get month.
    pub fn month(&self) -> u64 {
        self.0.minor
    }

    /// Get day.
    pub fn day(&self) -> u64 {
        self.0.patch
    }
}

impl std::fmt::Display for CalVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}.{:02}", self.0.major, self.0.minor)?;
        if self.0.patch > 0 {
            write!(f, ".{:02}", self.0.patch)?;
        }
        if !self.0.pre.is_empty() {
            write!(f, "-{}", self.0.pre)?;
        }
        if !self.0.build.is_empty() {
            write!(f, "+{}", self.0.build)?;
        }
        Ok(())
    }
}

impl std::ops::Deref for CalVer {
    type Target = Version;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
