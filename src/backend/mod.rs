use anyhow::Result;
use clap::Command;
use std::path::PathBuf;

trait Backend {
    fn runtime_data(&self) -> RuntimeData;

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

    fn metadata(self) -> Metadata {}

    fn cmd(self) -> Option<impl Iterator<Item = Command>> {
        match self {
            _ => None::<std::iter::Empty<Command>>,
        }
    }

    fn is(name: BackendType) -> Option<Box<dyn Backend>> {
        match name {
            _ => None,
        }
    }
}

pub struct Metadata {
    name: String,
    homepage: String,
    description: String,
    notes: String,
}

pub struct RuntimeData {
    installed_path: PathBuf,
}
