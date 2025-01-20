use clap::Command;
use std::path::PathBuf;

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
    pub fn from_str(s: &str) -> Self {
        match s {
            _ => BackendType::Unknown,
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
