use crate::{
    backend::BackendType, env::pulith::PulithEnv, reg::Reg, utils::task_pool::POOL,
    utils::ver::VersionKind,
};

use anyhow::{Result, bail};
use core::slice::SlicePattern;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
    path::PathBuf,
};
use tokio::{
    fs::{File, OpenOptions, rename},
    io::{AsyncWriteExt, BufReader},
};

use super::Cache;

pub static TOOL_REG: Lazy<ToolReg> = Lazy::new(|| ToolReg::load()?);

pub struct ToolRegAPI;

impl ToolRegAPI {
    pub fn get_names(bk: BackendType) -> Option<impl Iterator<Item = String>> {
        TOOL_REG
            .reg
            .iter()
            .find_map(&bk)
            .map(|info| info.keys().cloned())
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
