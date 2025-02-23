use crate::env::PulithEnv;

use anyhow::{Result, bail};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
    path::PathBuf,
};

pub static TOOL_REG: Lazy<ToolReg> = Lazy::new(|| ToolReg::load()?);

pub struct ToolRegAPI;

impl ToolRegAPI {
    pub fn get_names(bk: &BackendType) -> Option<impl Iterator<Item = String>> {
        TOOL_REG
            .reg
            .iter()
            .find(bk)
            .map(|(_, info)| info.keys().cloned()) // TODO: more info
    }
}

type ToolReg = Reg<HashMap<BackendType, ToolInfo>>;
type ToolInfo = BTreeMap<String, ToolStatus>;

impl Cache for ToolReg {
    fn locate() -> Result<PathBuf> {
        Ok(PulithEnv::new()?.store().root().join("tool.reg.lock"))
    }

    fn load() -> Result<Self>;

    fn save(&self) -> Result<()>;
}

impl Default for ToolReg {
    fn default() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ToolStatus {
    install_path: PathBuf,
    version: VersionKind,
    scope: Scope,
    checksum: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "kebab-case")]
enum Scope {
    Global,
    Local(Vec<PathBuf>),
    Hidden,
}

impl Default for Scope {
    fn default() -> Self {
        Self::Global
    }
}
