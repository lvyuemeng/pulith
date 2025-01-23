use std::fmt;
use std::ops::Deref;

use anyhow::{Context, Result, bail};
use regex::Regex;
use semver::Version;

pub enum VersionKind {
    SemVer(SemVer),
    CalVer(CalVer),
    Raw(PartRaw),
}

struct SemVer(Version);

impl Deref for SemVer {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.0)
    }
}

struct 


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PartRaw {
    major: u8,
    minor: u8,
    patch: u8,
    pre_release: Option<String>,
    build_metadata: Option<String>,
    lts: bool,
}

impl TryFrom<&str> for PartRaw {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.trim_end();
        let lts = value.ends_with("lts");
        let value = value.trim_end_matches("lts");

        let mut parts = value.split(&['.', '-', '+'][..]);
        let major = parse_ver(parts.next(), "major").unwrap_or(0);
        let minor = parse_ver(parts.next(), "minor").unwrap_or(0);
        let patch = parse_ver(parts.next(), "patch").unwrap_or(0);

        let pre = parse_pre(parts.next())?;
        let build = parse_build(parts.next())?;

        Ok(PartRaw {
            major,
            minor,
            patch,
            pre_release: pre,
            build_metadata: build,
            lts,
        })
    }
}

fn parse_ver(part: Option<&str>, component: &str) -> Result<u8> {
    part.with_context(|| format!("missing {} version", component))?
        .parse()
        .with_context(|| format!("invalid {} version", component))
}

fn parse_pre(part: Option<&str>) -> Result<Option<String>> {
    let re = Regex::new(r"(alpha|beta|rc)(\.\d+)?$").with_context(|| "regex error")?;
    let Some(part) = part else { return Ok(None) };
    if re.is_match(part) {
        Ok(Some(part.to_string()))
    } else {
        bail!("invalid prerelease version: {}", part)
    }
}

fn parse_build(part: Option<&str>) -> Result<Option<String>> {
    let re = Regex::new(r"^(build|sha)\.\w+$").with_context(|| "regex error")?;
    let Some(part) = part else { return Ok(None) };
    if re.is_match(part) {
        Ok(Some(part.to_string()))
    } else {
        bail!("invalid build metadata: {}", part)
    }
}
