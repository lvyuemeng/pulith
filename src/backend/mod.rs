use anyhow::Result;
use clap::Command;
use std::{path::PathBuf, time::SystemTime};

use crate::tool::ver::VersionKind;

trait Backend {
    fn runtime_snap() -> Snap;
    fn metadata() -> Metadata;
    fn env_vars() -> impl Iterator<Item = String>;
    fn cmd() -> Option<impl Iterator<Item = Command>>;

    // Tool ops
    fn add(arg: AddArg) -> Result<()>;
    fn use_ver(arg: UseArg) -> Result<()>;
    fn remove(arg: RmArg) -> Result<()>;
    fn list(arg: ListArg) -> Result<()>;
    fn update(arg: UpdateArg) -> Result<()>;
    fn search(arg: SearchArg) -> Result<()>;
}

pub struct CheckReg;
pub struct Ops;
pub struct UpdateReg;

#[derive(Debug, Clone, Copy)]
pub enum BackendType {
    Unknown,
}

impl BackendType {
    pub fn from_str(s: &str) -> Self {
        match s {
            _ => BackendType::Unknown,
        }
    }
}

impl Backend for BackendType {
    fn cmd() -> Option<impl Iterator<Item = Command>> {
        None
    }
}

pub struct Metadata {
    name: String,
    homepage: String,
    description: String,
    notes: String,
}

pub struct Snap {
    install_path: PathBuf,
    before: SystemTime,
    version: VersionKind,
}
