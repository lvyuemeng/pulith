use std::path::PathBuf;

use anyhow::Result;
use clap::Command;

trait Backend {
    fn metadata(&self) -> Metadata;
    fn runtime_data(&self) -> RuntimeData;
    fn cmd() -> Option<impl Iterator<Item = Command>> {
        None::<std::iter::Empty<Command>>
    }

    // Tool ops
    fn add();
    fn use_ver();
    fn remove();
    fn list();
    fn update();
    fn search();
}

pub struct CheckReg;
pub struct Ops;
pub struct UpdateReg;

#[derive(Debug, Clone, Copy)]
pub enum BackendType {
    Unknown,
}

impl BackendType {
    fn from_str(s: &str) -> Self {
        match s {
            _ => BackendType::Unknown,
        }
    }

    fn is(name: BackendType) -> Option<&'static dyn Backend> {
        match name {
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackendTool {
    bk: BackendType,
    name: String,
}

fn parse_tool(s: &str) -> Result<BackendTool> {
    let (backend_str, tool) = s.split_once(":")?;
    let bk = BackendType::from_str(backend_str);
    let name = tool.to_string();
    Ok(BackendTool { bk, name })
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
