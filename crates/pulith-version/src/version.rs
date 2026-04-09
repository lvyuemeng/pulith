//! Version types and operations.

use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version as SemVer;
use serde::{Deserialize, Serialize};
use thiserror::Error;

static CALVER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?<year>[0-9]{4})[-.](?<month>(0?[1-9]|10|11|12))(?:\.(?<day>(0?[1-9]|[1-3][0-9])))?(?:\+(?<micro>[0-9]+))?(?:-(?<pre>[a-zA-Z][-0-9a-zA-Z.]+))?$").unwrap()
});

static PARTIAL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?<major>[0-9]+))?(?:\.(?<minor>[0-9]+))?(?:\.(?<patch>[0-9]+))?(?:-(?<pre>[a-zA-Z][-0-9a-zA-Z.]*))?(?:\+(?<build>[-0-9a-zA-Z.]+))?(?:lts)?$").unwrap()
});

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("invalid semver")]
    SemVer(#[from] semver::Error),
    #[error(transparent)]
    CalVer(#[from] CalVerError),
    #[error(transparent)]
    Partial(#[from] PartialError),
    #[error("unknown version scheme")]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionKindType {
    SemVer,
    CalVer,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VersionKind {
    SemVer(SemVer),
    CalVer(CalVer),
    Partial(Partial),
}

#[derive(Debug, Error)]
#[error("invalid CalVer format: {0}")]
pub struct CalVerError(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CalVer(SemVer);

impl CalVer {
    pub fn parse(s: &str) -> Result<Self, CalVerError> {
        let caps = CALVER_REGEX
            .captures(s)
            .ok_or_else(|| CalVerError(s.to_string()))?;

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
            SemVer::parse(&version).map_err(|_| CalVerError(s.to_string()))?,
        ))
    }

    pub fn from_ymd(year: u64, month: u64, day: u64) -> Result<Self, CalVerError> {
        if !(1..=12).contains(&month) {
            return Err(CalVerError(format!("invalid month: {}", month)));
        }
        if !(1..=31).contains(&day) {
            return Err(CalVerError(format!("invalid day: {}", day)));
        }

        let version = format!("{:04}.{:02}.{:02}", year, month, day);
        Ok(Self(
            SemVer::parse(&version).map_err(|_| CalVerError(version))?,
        ))
    }

    pub fn year(&self) -> u64 {
        self.0.major
    }
    pub fn month(&self) -> u64 {
        self.0.minor
    }
    pub fn day(&self) -> u64 {
        self.0.patch
    }
}

impl std::str::FromStr for CalVer {
    type Err = CalVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CalVer::parse(s)
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
    type Target = SemVer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Error)]
#[error("invalid partial version: {0}")]
pub struct PartialError(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Partial {
    pub major: Option<u64>,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
    pub lts: bool,
}

impl Partial {
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

    pub fn matches(&self, version: &VersionKind) -> bool {
        match version {
            VersionKind::SemVer(v) => {
                self.major.is_none_or(|m| m == v.major)
                    && self.minor.is_none_or(|m| m == v.minor)
                    && self.patch.is_none_or(|m| m == v.patch)
            }
            VersionKind::CalVer(v) => {
                self.major.is_none_or(|m| m == v.year())
                    && self.minor.is_none_or(|m| m == v.month())
                    && self.patch.is_none_or(|m| m == v.day())
            }
            VersionKind::Partial(other) => {
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

impl std::str::FromStr for VersionKind {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(v) = s.parse::<SemVer>() {
            return Ok(VersionKind::SemVer(v));
        }
        if let Ok(v) = s.parse::<CalVer>() {
            return Ok(VersionKind::CalVer(v));
        }
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
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        s.parse()
    }

    pub fn as_semver(&self) -> Option<&SemVer> {
        match self {
            VersionKind::SemVer(v) => Some(v),
            _ => None,
        }
    }

    pub fn kind(&self) -> VersionKindType {
        match self {
            VersionKind::SemVer(_) => VersionKindType::SemVer,
            VersionKind::CalVer(_) => VersionKindType::CalVer,
            VersionKind::Partial(_) => VersionKindType::Partial,
        }
    }
}

#[derive(Debug, Error)]
pub enum VersionPreferenceError {
    #[error("invalid version requirement: {0}")]
    InvalidRequirement(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionRequirement {
    Any,
    Exact(VersionKind),
    Partial(Partial),
    SemVer(semver::VersionReq),
}

impl VersionRequirement {
    pub fn parse(input: &str) -> Result<Self, VersionPreferenceError> {
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed == "*" {
            return Ok(Self::Any);
        }

        if let Ok(requirement) = semver::VersionReq::parse(trimmed) {
            return Ok(Self::SemVer(requirement));
        }

        if let Ok(partial) = Partial::parse(trimmed) {
            return Ok(Self::Partial(partial));
        }

        if let Ok(version) = VersionKind::parse(trimmed) {
            return Ok(Self::Exact(version));
        }

        Err(VersionPreferenceError::InvalidRequirement(
            trimmed.to_string(),
        ))
    }

    pub fn matches(&self, version: &VersionKind) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => expected == version,
            Self::Partial(partial) => partial.matches(version),
            Self::SemVer(requirement) => version
                .as_semver()
                .is_some_and(|value| requirement.matches(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionPreference {
    Latest,
    Lowest,
    HighestStable,
    Lts,
    Pinned(VersionKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionPolicy {
    pub requirement: VersionRequirement,
    pub preference: VersionPreference,
}

impl Default for SelectionPolicy {
    fn default() -> Self {
        Self {
            requirement: VersionRequirement::Any,
            preference: VersionPreference::Latest,
        }
    }
}

pub fn select_preferred<'a>(
    versions: &'a [VersionKind],
    policy: &SelectionPolicy,
) -> Option<&'a VersionKind> {
    let candidates = matching_candidates(versions, &policy.requirement);

    if candidates.is_empty() {
        return None;
    }

    match &policy.preference {
        VersionPreference::Lowest => candidates.into_iter().min(),
        VersionPreference::Latest => candidates.into_iter().max(),
        VersionPreference::HighestStable => candidates
            .into_iter()
            .filter(|version| version.is_stable())
            .max()
            .or_else(|| {
                matching_candidates(versions, &policy.requirement)
                    .into_iter()
                    .max()
            }),
        VersionPreference::Lts => candidates
            .into_iter()
            .filter(|version| matches!(version, VersionKind::Partial(partial) if partial.lts))
            .max()
            .or_else(|| {
                matching_candidates(versions, &policy.requirement)
                    .into_iter()
                    .max()
            }),
        VersionPreference::Pinned(version) => {
            versions.iter().find(|candidate| *candidate == version)
        }
    }
}

fn matching_candidates<'a>(
    versions: &'a [VersionKind],
    requirement: &VersionRequirement,
) -> Vec<&'a VersionKind> {
    versions
        .iter()
        .filter(|version| requirement.matches(version))
        .collect()
}

impl VersionKind {
    pub fn is_stable(&self) -> bool {
        match self {
            VersionKind::SemVer(version) => version.pre.is_empty(),
            VersionKind::CalVer(version) => version.pre.is_empty(),
            VersionKind::Partial(version) => version.pre_release.is_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Partial, SelectionPolicy, VersionKind, VersionKindType, VersionPreference,
        VersionRequirement, select_preferred,
    };

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

    #[test]
    fn semver_requirement_matches_semver() {
        let requirement = VersionRequirement::parse("^1.2").unwrap();
        assert!(requirement.matches(&VersionKind::parse("1.2.3").unwrap()));
        assert!(!requirement.matches(&VersionKind::parse("2.0.0").unwrap()));
    }

    #[test]
    fn partial_requirement_matches_cross_scheme() {
        let requirement = VersionRequirement::Partial(Partial::parse("2024.01").unwrap());
        assert!(requirement.matches(&VersionKind::parse("2024.01.15").unwrap()));
    }

    #[test]
    fn select_latest_prefers_highest_match() {
        let versions = vec![
            VersionKind::parse("1.2.3").unwrap(),
            VersionKind::parse("1.3.0").unwrap(),
            VersionKind::parse("1.2.9").unwrap(),
        ];

        let selected = select_preferred(
            &versions,
            &SelectionPolicy {
                requirement: VersionRequirement::parse("^1.2").unwrap(),
                preference: VersionPreference::Latest,
            },
        )
        .unwrap();

        assert_eq!(selected.to_string(), "1.3.0");
    }

    #[test]
    fn select_highest_stable_skips_prerelease() {
        let versions = vec![
            VersionKind::parse("1.2.3-alpha.1").unwrap(),
            VersionKind::parse("1.2.2").unwrap(),
        ];

        let selected = select_preferred(
            &versions,
            &SelectionPolicy {
                requirement: VersionRequirement::Any,
                preference: VersionPreference::HighestStable,
            },
        )
        .unwrap();

        assert_eq!(selected.to_string(), "1.2.2");
    }

    #[test]
    fn pinned_preference_returns_exact_version() {
        let pinned = VersionKind::parse("20.12.1").unwrap();
        let versions = vec![pinned.clone(), VersionKind::parse("20.11.0").unwrap()];

        let selected = select_preferred(
            &versions,
            &SelectionPolicy {
                requirement: VersionRequirement::Any,
                preference: VersionPreference::Pinned(pinned),
            },
        )
        .unwrap();

        assert_eq!(selected.to_string(), "20.12.1");
    }
}
