use std::{fmt, str::FromStr};

use crate::backend::BackendType;
use anyhow::bail;
use pulith_core::ver::VersionKind;

#[derive(Debug, Clone)]
enum Descriptor {
    Backend(BackendType),
    Package(Package),
    List(usize),
}

#[derive(Debug, Clone)]
struct Package {
    backend: Option<BackendType>,
    name: String,
    ver: Option<VersionKind>,
}

impl FromStr for Descriptor {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(num) = s.parse() {
            return Ok(Descriptor::List(num));
        }
        if let Some(pkg_spec) = s.strip_prefix('@') {
            let mut parts = pkg_spec.splitn(3, ':');
            return match (parts.next(), parts.next(), parts.next()) {
                (Some(bk), Some(name), ver) => {
                    let bk = BackendType::from_str(bk)?;
                    Ok(Descriptor::Package(Package {
                        backend: Some(bk),
                        name: name.to_string(),
                        ver: ver.map(|s| VersionKind::from_str(s)).transpose()?,
                    }))
                }
                _ => bail!("..."),
            };
        }

        let mut parts = s.splitn(2, ':');
        return match (parts.next(), parts.next()) {
            (Some(name), ver) => Ok(Descriptor::Package(Package {
                backend: None,
                name: name.to_string(),
                ver: ver.map(|s| VersionKind::from_str(s)).transpose()?,
            })),
            _ => bail!("..."),
        };
    }
}

impl fmt::Display for Descriptor {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Backend(bk) => {
				write!(f,"{}",bk.as_ref())
			},
            Self::List(num) => {
                write!(f,"{num}")
            }
            Self::Package(p) => {
                write!(f,"{p}")
            }
		}
	}
}

impl Package {
    pub fn id(&self) -> String {
        let sanitize = |s: &str| {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>()
                .to_lowercase()
        };

        let bk_ = self.backend.map(|bk| sanitize(bk.into()));
        let name_ = sanitize(&self.name);

        if let Some(bk_) = bk_ {
            format!("{}.{}", bk_, name_)
        } else {
            format!("unknown.{}", name_)
        }
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.id();
        write!(f,"{s}")
    }
}

