use std::{collections::HashMap, hash::Hash, str::FromStr};

use serde::{Deserialize, Deserializer};
use thiserror::Error;

pub trait Context: Eq + Hash + AsRef<str> {}

#[derive(Debug, Deserialize)]
pub struct FlagConfig<Command: Context, Backend: Context> {
    #[serde(default)]
    pub policy: FlagPolicy,
    #[serde(rename = "flag", default)]
    pub cmd_flags: HashMap<Command, FlagMap<Backend>>,
}

type FlagMap<Backend: Context> = HashMap<FlagName, FlagResolve<Backend>>;

#[derive(Debug, Deserialize)]
pub struct FlagResolve<Backend: Context> {
    pub global: FlagValue,
    pub backend: HashMap<Backend, FlagValue>,
}

// TODO: Context Plugin
// Trait ContextProvider { const KEYS: &'static [&'static str]; fn validate()}
// check context validation and is in the set of context keys

// TODO: impl parser for --flags|*
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FlagName {
    pub names: Vec<String>,
}

#[derive(Debug, Error)]
pub enum FlagError {
    #[error("Invalid flag name: {0}")]
    ParseFlagName(String),
}

impl FromStr for FlagName {
    type Err = FlagError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pat: Vec<String> = s
            .split('|')
            .map(|w| w.trim_start_matches('-').to_string())
            .collect();
        if pat.is_empty() {
            return Err(FlagError::ParseFlagName(s.to_string()));
        }
        Ok(FlagName { names: pat })
    }
}

impl<'de> Deserialize<'de> for FlagName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        let flag_names = FlagName::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(flag_names)
    }
}

#[derive(Debug, Deserialize)]
pub struct FlagValue {
    pub pat: Option<Vec<String>>,
    pub arg_pat: ArgPat,
}

#[derive(Debug, Deserialize)]
pub struct ArgPat {
    pub pat: Option<String>,
    pub default: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FlagPolicy {
    #[serde(rename = "undefined-policy")]
    pub undefined: UndefinedPolicy,
    #[serde(rename = "default-policy")]
    pub bk_default: ConflictPolicy,
    #[serde(rename = "global-default-policy")]
    pub global_default: ConflictPolicy,
}

impl Default for FlagPolicy {
    fn default() -> Self {
        Self {
            undefined: UndefinedPolicy::Ignore,
            bk_default: ConflictPolicy::Override,
            global_default: ConflictPolicy::Inherit,
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum UndefinedPolicy {
    Ignore,
    Pass,
    Error,
}

#[derive(Debug, Deserialize)]
pub enum ConflictPolicy {
    Inherit,
    Override,
}
