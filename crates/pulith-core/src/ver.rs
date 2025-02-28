use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::{fmt, str::FromStr};
use thiserror::Error;

static CALVER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?<year>[0-9]{1,4})-(?<month>((0?[1-9]{1})|10|11|12))(-(?<day>(0?[1-9]{1}|[1-3]{1}[0-9]{1})))?((_|\.)(?<micro>[0-9]+))?(?<pre>-[a-zA-Z]{1}[-0-9a-zA-Z.]+)?$").unwrap()
});

static PARTIAL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([^.]*)(?:\.([^.]*))?(?:\.([^.]*))?(?:[+\./-](.*))?$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VersionKind {
    SemVer(SemVer),
    CalVer(CalVer),
    Partial(Partial),
}

#[derive(Error, Debug)]
pub enum VersionKindError {
    #[error(transparent)]
    CalVer(#[from] CalVerError),
    #[error(transparent)]
    SemVer(#[from] semver::Error),
    #[error(transparent)]
    Partial(#[from] PartialError),
}

impl FromStr for VersionKind {
    type Err = VersionKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(v) = s.parse::<CalVer>() {
            Ok(VersionKind::CalVer(v))
        } else if let Ok(v) = Version::parse(s) {
            Ok(VersionKind::SemVer(SemVer(v)))
        } else {
            Ok(VersionKind::Partial(
                s.parse::<Partial>()?,
            ))
        }
    }
}

impl fmt::Display for VersionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionKind::SemVer(v) => write!(f, "{}", v),
            VersionKind::CalVer(v) => write!(f, "{}", v),
            VersionKind::Partial(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SemVer(Version);

impl Deref for SemVer {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CalVer(Version);

#[derive(Error, Debug)]
pub enum CalVerError {
    #[error("Parse Calendar version error: {0:?}")]
    InvalidFormat(String),
}

impl FromStr for CalVer {
    type Err = CalVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let caps = CALVER_REGEX
            .captures(s)
            .ok_or(CalVerError::InvalidFormat(s.to_string()))?;

        let year = caps
            .name("year")
            .map(|cap| cap.as_str().trim_start_matches('0'))
            .unwrap_or("0");
        let year = if year.len() < 4 {
            format!("20{}", year) // Adjust 2-digit year to 4-digit
        } else {
            year.to_string()
        };

        let month = caps
            .name("month")
            .map(|cap| cap.as_str().trim_start_matches('0'))
            .unwrap_or("0");

        let day = caps
            .name("day")
            .map(|cap| cap.as_str().trim_start_matches('0'))
            .unwrap_or("0");

        let mut version = format!("{year}.{month}.{day}");

        if let Some(pre) = caps.name("pre") {
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
}

impl Deref for CalVer {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for CalVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ver = self.deref();

        write!(f, "{:0>4}-{:0>2}", ver.major, ver.minor)?;

        if ver.patch > 0 {
            write!(f, "-{:0>2}", ver.patch)?;
        }

        if !ver.build.is_empty() {
            write!(f, ".{}", ver.build)?;
        }

        if !ver.pre.is_empty() {
            write!(f, "-{}", ver.pre)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Partial {
    pub major: Option<String>,
    pub minor: Option<String>,
    pub patch: Option<String>,
    pub other: Option<String>,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
    pub lts: bool,
}

#[derive(Debug, Error)]
#[error("Parse Partial version error: {msg}")]
pub struct PartialError {
    msg: String,
}

impl FromStr for Partial {
    type Err = PartialError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim_end();
        let lts = value.ends_with("lts");
        let value = value.trim_end_matches("lts");

        // Split into [core_and_pre] and [build_metadata] (if any)
        let (parts, build) = value
            .split_once('+')
            .map(|(core, build)| (core, Some(build)))
            .unwrap_or((value, None));

        // Split into [version_core] and [pre_release] (if any)
        let (parts, pre) = parts
            .split_once('-')
            .map(|(core, pre)| (core, Some(pre)))
            .unwrap_or((parts, None));

        let (major, minor, patch, other) = if let Some(caps) = PARTIAL_REGEX.captures(parts) {
            let major = caps.get(1).map(|m| m.as_str().to_string());
            let minor = caps.get(2).map(|m| m.as_str().to_string());
            let patch = caps.get(3).map(|m| m.as_str().to_string());
            let other = caps.get(4).map(|m| m.as_str().to_string());

            (major, minor, patch, other)
        } else {
            Err(PartialError {
                msg: value.to_string(),
            })?
        };

        Ok(Partial {
            major,
            minor,
            patch,
            other,
            pre_release: pre.map(|s| s.to_string()),
            build_metadata: build.map(|s| s.to_string()),
            lts,
        })
    }
}

impl fmt::Display for Partial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(major) = &self.major {
            write!(f, "{}.", major)?;
        }
        if let Some(minor) = &self.minor {
            write!(f, "{}.", minor)?;
        }
        if let Some(patch) = &self.patch {
            write!(f, "{}.", patch)?;
        }

        if let Some(other) = &self.other {
            write!(f, "-{}", other)?;
        }

        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }

        if let Some(build) = &self.build_metadata {
            write!(f, "+{}", build)?;
        }

        if self.lts {
            write!(f, " lts")?;
        }

        Ok(())
    }
}
