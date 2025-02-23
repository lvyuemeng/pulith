pub mod install;

use anyhow::Result;
use clap::Command;
use std::{fmt, path::PathBuf, time::SystemTime};

pub trait Backend {
    fn snap(&self) -> Option<Snap> {
        None
    }
    fn metadata(&self) -> Metadata;
    fn env_vars(&self) -> Option<impl Iterator<Item = String>> {
        None
    }
    fn cmd(&self) -> Option<impl Iterator<Item = Command>> {
        None
    }
    fn tools(&self) -> Option<impl Iterator<Item = String>> {
        None
    }

    // Tool ops
    fn add(&self, arg: AddArg) -> Result<()>;
    fn use_ver(&self, arg: UseArg) -> Result<()>;
    fn remove(&self, arg: RmArg) -> Result<()>;
    fn list(&self, arg: ListArg) -> Result<()>;
    fn update(&self, arg: UpdateArg) -> Result<()>;
    fn search(&self, arg: SearchArg) -> Result<()>;
}

pub struct Metadata {
    name: String,
    homepage: String,
    description: String,
    notes: String,
}

pub struct Snap {
    install_path: PathBuf,
    env_var: (String, PathBuf),
    version: VersionKind,
    before: SystemTime,
}

pub struct CheckReg;
pub struct Ops;
pub struct UpdateReg;

#[derive(Debug, Clone, Copy)]
pub enum BackendType {
    Unknown,
    // Linux Native
    Apt,
    Dnf,
    Pacman,
    Zypper,
    Apk,
    // Macos
    Brew,
    // Windows Native
    Winget,
    Scoop,
    Choco,
}

impl BackendType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "apt" => BackendType::Apt,
            "dnf" => BackendType::Dnf,
            "pacman" => BackendType::Pacman,
            "zypper" => BackendType::Zypper,
            "apk" => BackendType::Apk,
            "brew" => BackendType::Brew,
            "winget" => BackendType::Winget,
            "scoop" => BackendType::Scoop,
            "choco" => BackendType::Choco,
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

    fn tools(&self) -> Option<impl Iterator<Item = String>> {
        match &self {
            BackendType::Unknown => todo!(),
        }
    }
}

    pub fn which_pm() -> Option<BackendType> {
        match SYSTEM_INFO.os {
            OS::Macos => Some(BackendType::Brew),
            OS::Windows => Some(BackendType::Winget),
            OS::Linux(distro) => match distro {
                Linux::Debian => Some(BackendType::Apt),
                Linux::Ubuntu => Some(BackendType::Apt),
                Linux::LinuxMint => Some(BackendType::Apt),
                Linux::KaliLinux => Some(BackendType::Apt),
                Linux::Fedora => Some(BackendType::Dnf),
                Linux::RedHatEnterpriseLinux => Some(BackendType::Dnf),
                Linux::ArchLinux => Some(BackendType::Pacman),
                Linux::Manjaro => Some(BackendType::Pacman),
                Linux::OpenSUSE => Some(BackendType::Zypper),
                Linux::AlpineLinux => Some(BackendType::Apk),
                _ => None,
            },
            _ => None,
        }
    }

