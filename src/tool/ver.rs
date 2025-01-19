use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionKey {
    ver: Version,
    key: String,
}

impl TryFrom<&str> for VersionKey {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (ver, key) = value
            .split_once('-')
            .ok_or_else(|| anyhow!("invalid version key: {}", value))?;
        Ok(VersionKey {
            ver: Version::try_from(ver).with_context(|| format!("invalid version: {}", ver))?,
            key: key.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u8,
    minor: u8,
    patch: u8,
    pre_release: Option<String>,
    build_metadata: Option<String>,
    lts: bool,
}

impl TryFrom<&str> for Version {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.trim_end();
        let lts = value.ends_with("lts");
        let value = value.trim_end_matches("lts");

        let mut parts = value.split(&['.', '-', '+'][..]);
        let major = parse_ver(parts.next(), "major")?;
        let minor = parse_ver(parts.next(), "minor")?;
        let patch = parse_ver(parts.next(), "patch")?;

        let pre = parse_pre(parts.next())?;
        let build = parse_build(parts.next())?;

        Ok(Version {
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
