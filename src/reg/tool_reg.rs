use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
};

use crate::{backend::BackendType, env::pulith::PulithEnv, reg::Reg, tool::ver::VersionKind};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

static TOOL_REG: Lazy<ToolReg> = Lazy::new(|| ToolRegAPI::load()?);

pub struct ToolRegAPI;

impl Reg<ToolReg> for ToolRegAPI {
    fn load() -> Result<ToolReg> {
        // improve...
        let cur_env = PulithEnv::new()?.store().root().join("reg.toml");

        let data = std::fs::read_to_string(&cur_env)?;
        let reg: ToolReg = serde::Deserialize::deserialize(&data)?;

        Ok(reg)
    }
}

impl ToolRegAPI {
    fn get(&self, tool: &str) -> Option<ToolStatus> {
        TOOL_REG.deref().get(tool).cloned()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolReg(HashMap<BackendType, ToolInfo>);

impl ToolReg {
    pub fn get(&self, tool: &str) -> Option<&ToolStatus> {
        self.deref().values().find_map(|v| v.get(tool))
    }
}

impl Deref for ToolReg {
    type Target = HashMap<BackendType, ToolInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ToolReg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolInfo(BTreeMap<String, ToolStatus>);

impl Deref for ToolInfo {
    type Target = BTreeMap<String, ToolStatus>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ToolInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolStatus {
    install_path: PathBuf,
    version: VersionKind,
    scope: Scope,
}

#[derive(Debug)]
enum Scope {
    Global,
    Local(Vec<PathBuf>),
    Hidden,
}
