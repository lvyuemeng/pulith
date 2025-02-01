use anyhow::Result;
use clap::Command;
use std::{fmt, path::PathBuf, time::SystemTime};

pub trait Backend {
    fn snap(self) -> Option<Snap>;
    fn metadata(self) -> Metadata;
    fn env_vars(self) -> impl Iterator<Item = String>;
    fn cmd(self) -> Option<impl Iterator<Item = Command>>;
    fn tools(self) -> impl Iterator<Item = String>;

    // Tool ops
    fn add(self, arg: AddArg) -> Result<()>;
    fn use_ver(self, arg: UseArg) -> Result<()>;
    fn remove(self, arg: RmArg) -> Result<()>;
    fn list(self, arg: ListArg) -> Result<()>;
    fn update(self, arg: UpdateArg) -> Result<()>;
    fn search(self, arg: SearchArg) -> Result<()>;
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

    pub fn all() -> impl Iterator<Item = BackendType> {
        // TODO: os compatible
        [BackendType::Unknown].iter()
    }
}

impl Backend for BackendType {
    fn cmd() -> Option<impl Iterator<Item = Command>> {
        None
    }

    fn snap() -> Option<Snap> {
        todo!()
    }

    fn metadata() -> Metadata {
        todo!()
    }

    fn env_vars() -> impl Iterator<Item = String> {
        todo!()
    }

    fn add(arg: AddArg) -> Result<()> {
        todo!()
    }

    fn use_ver(arg: UseArg) -> Result<()> {
        todo!()
    }

    fn remove(arg: RmArg) -> Result<()> {
        todo!()
    }

    fn list(arg: ListArg) -> Result<()> {
        todo!()
    }

    fn update(arg: UpdateArg) -> Result<()> {
        todo!()
    }

    fn search(arg: SearchArg) -> Result<()> {
        todo!()
    }

    fn tools(self) -> Option<impl Iterator<Item = String>> {
        match self {
            BackendType::Unknown => todo!(),
        }
    }
}
