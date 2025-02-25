use crate::backend::BackendType;

use pulith_core::{reg::RegLoader, utils::ver::VersionKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

pub struct ToolRegAPI;

impl ToolRegAPI {
    pub fn get_names(reg: &ToolRegLoader,bk: &BackendType) -> Option<impl Iterator<Item = String>> {
        reg.deref()
            .iter()
            .find(bk)
            .map(|(_, info)| info.keys().cloned()) // TODO: more info
    }
}

type ToolRegLoader = RegLoader<ToolInfo>;
type ToolStorage = HashMap<BackendType, ToolInfo>;
type ToolInfo = BTreeMap<String, ToolStatus>;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
