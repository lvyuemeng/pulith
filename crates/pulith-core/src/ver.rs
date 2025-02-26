use std::ops::Deref;
use std::{fmt, str::FromStr};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};

fn parse_num(part: Option<&str>) -> u32 {
    part.unwrap_or("0").parse().unwrap_or(0)
}

static CALVER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?<year>[0-9]{1,4})-(?<month>((0?[1-9]{1})|10|11|12))(-(?<day>(0?[1-9]{1}|[1-3]{1}[0-9]{1})))?((_|\.)(?<micro>[0-9]+))?(?<pre>-[a-zA-Z]{1}[-0-9a-zA-Z.]+)?$").unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord,Serialize,Deserialize)]
pub enum VersionKind {
    SemVer(SemVer),
    CalVer(CalVer),
    Partial(Partial),
}

impl FromStr for VersionKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(v) = s.parse::<CalVer>() {
            Ok(VersionKind::CalVer(v))
        } else if let Ok(v) = Version::parse(s) {
            Ok(VersionKind::SemVer(SemVer(v)))
        } else {
            Ok(VersionKind::Partial(s.parse()?))
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

impl FromStr for CalVer {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let caps = CALVER_REGEX
            .captures(s)
            .with_context(|| format!("Invalid CalVer: {}", s))?;

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

        Ok(Self(Version::parse(&version)?))
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord,Serialize,Deserialize)]
pub struct Partial {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
    pub other: Option<String>,
    pub lts: bool,
}

impl FromStr for Partial {
    type Err = anyhow::Error;

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

        let mut parts = parts.split(".");
        let major = parse_num(parts.next());
        let minor = parse_num(parts.next());
        let patch = parse_num(parts.next());

        let other = if parts.next().is_none() {
            None
        } else {
            Some(parts.collect())
        };

        Ok(Partial {
            major,
            minor,
            patch,
            pre_release: pre.map(|s| s.to_string()),
            build_metadata: build.map(|s| s.to_string()),
            other: other,
            lts,
        })
    }
}

impl fmt::Display for Partial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

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
