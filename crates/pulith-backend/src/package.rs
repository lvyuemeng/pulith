use crate::backend::BackendType;

use pulith_core::ver::VersionKind;
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum Descriptor<'a> {
    Backend(BackendType),
    Package(Package<'a>),
    List(usize),
}

#[derive(Debug, Clone)]
pub struct Package<'a> {
    backend: Option<BackendType>,
    args: Vec<&'a str>,
    ver: Option<VersionKind>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid backend type: {0}")]
    InvalidBackend(String),
    #[error("invalid version specification: {0}")]
    InvalidVersion(String),
    #[error("missing backend specification after '@'")]
    MissingBackend,
    #[error("missing arguments for package descriptor")]
    MissingArgs,
    #[error("invalid descriptor format")]
    InvalidFormat,
}

impl FromStr for Descriptor<'_> {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(num) = s.parse::<usize>() {
            return Ok(Descriptor::List(num));
        }

        let (backend, spec) = if let Some(stripped) = s.strip_prefix('@') {
            if let Some((b_str, rest)) = stripped.split_once(':') {
                let backend = BackendType::from_str(b_str)
                    .map_err(|e| ParseError::InvalidBackend(e.to_string()))?;
                (Some(backend), rest)
            } else {
                let backend = BackendType::from_str(stripped)
                    .map_err(|e| ParseError::InvalidBackend(e.to_string()))?;
                return Ok(Descriptor::Backend(backend));
            }
        } else {
            (None, s)
        };

        let (args_str, ver_str) = split_args_ver(spec);
        let args: Vec<&str> = args_str.split_whitespace().collect();
        if args.is_empty() {
            return Err(ParseError::MissingArgs);
        }
        let ver = ver_str
            .map(|v| {
                VersionKind::from_str(v).map_err(|e| ParseError::InvalidVersion(e.to_string()))
            })
            .transpose()?;
        Ok(Descriptor::Package(Package { backend, args, ver }))
    }
}

// Uses `rsplit_once` to split at the last occurrence of ':' for version specification.
fn split_args_ver(s: &str) -> (&str, Option<&str>) {
    s.rsplit_once(':')
        .map_or((s, None), |(args, ver)| (args, Some(ver)))
}

impl fmt::Display for Descriptor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(bk) => {
                write!(f, "{}", bk.as_ref())
            }
            Self::List(num) => {
                write!(f, "{num}")
            }
            Self::Package(p) => {
                write!(f, "{p}")
            }
        }
    }
}

impl<'a> Package<'a> {
    pub fn args(&self) -> &[&'a str] {
        &self.args
    }
    pub fn name(&self) -> &str {
        assert!(!self.args.is_empty());
        &self.args[0]
    }
    pub fn ver(&self) -> Option<&VersionKind> {
        self.ver.as_ref()
    }
    pub fn id(&self) -> String {
        let sanitize = |s: &str| {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>()
                .to_lowercase()
        };

        let bk_ = self.backend.map(|bk| sanitize(bk.into()));
        let name_ = sanitize(&self.name());

        match bk_ {
            Some(bk) => format!("{}.{}", bk, name_),
            None => format!("unknown.{}", name_),
        }
    }
}

impl fmt::Display for Package<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_args = self.args.join(" ");
        let display_ver = self
            .ver
            .as_ref()
            .map_or(String::new(), |v| format!(":{}", v));
        let display_backend = self
            .backend
            .as_ref()
            .map_or(String::new(), |b| format!("@{}:", b.as_ref()));
        write!(
            f,
            "{}{}{}",
            display_backend,
            display_args.replace(':', "\\:"),
            display_ver
        )
    }
}
