mod format;
mod install;
mod reg;
pub mod winget;

use anyhow::{Result, bail};
use clap::Command;
use pulith_core::{
    env::{Linux, SystemInfoAPI, OS},
    ver::VersionKind,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, path::PathBuf, str::FromStr, time::SystemTime};

pub trait Backend {
    fn snap(&self) -> Option<Snap> {
        None
    }
    fn metadata(&self) -> Metadata;
    fn env_vars<T>(&self) -> Option<T>
    where
        T: Iterator<Item = String>,
    {
        None
    }
    fn cmd<T>(&self) -> Option<T>
    where
        T: Iterator<Item = Command>,
    {
        None
    }
}

trait Add {
    type Ctx;
    fn add(&self, ctx: Self::Ctx) -> Result<()>;
}

trait UseVer {
    type Ctx;
    fn use_ver(&self, ctx: Self::Ctx) -> Result<()>;
}

trait Remove {
    type Ctx;
    fn remove(&self, ctx: Self::Ctx) -> Result<()>;
}

trait List {
    type Ctx;
    // TODO!
    fn list(&self, ctx: Self::Ctx) -> Result<Vec<String>>;
}

trait Update {
    type Ctx;
    fn update(&self, ctx: Self::Ctx) -> Result<()>;
}

trait Search {
    type Ctx;
    // TODO!
    fn search(&self, ctx: Self::Ctx) -> Result<Vec<String>>;
}

pub struct Metadata {
    id: String,
    homepage: String,
    description: String,
    notes: Option<String>,
}

impl Metadata {
    pub fn new(id: &str, homepage: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            homepage: homepage.to_string(),
            description: description.to_string(),
            notes: None,
        }
    }

    pub fn with_notes(self, notes: &str) -> Self {
        Self {
            notes: Some(notes.to_string()),
            ..self
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snap {
    install_path: PathBuf,
    env_var: HashMap<String, Vec<String>>,
    version: VersionKind,
    before: SystemTime,
}

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
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
    pub fn which_pm() -> Option<BackendType> {
        match SystemInfoAPI::os() {
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
}

impl FromStr for BackendType {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<BackendType> {
        match value {
            "apt" => Ok(BackendType::Apt),
            "dnf" => Ok(BackendType::Dnf),
            "pacman" => Ok(BackendType::Pacman),
            "zypper" => Ok(BackendType::Zypper),
            "apk" => Ok(BackendType::Apk),
            "brew" => Ok(BackendType::Brew),
            "winget" => Ok(BackendType::Winget),
            "scoop" => Ok(BackendType::Scoop),
            "choco" => Ok(BackendType::Choco),
            "" => Ok(BackendType::Unknown),
            _ => bail!("..."),
        }
    }
}
impl AsRef<str> for BackendType {
    fn as_ref(&self) -> &str {
        match self {
            BackendType::Apk => "apt",
            // TODO!
            _ => "",
        }
    }
}
impl Into<&str> for BackendType {
    fn into(self) -> &'static str {
        match self {
            BackendType::Apk => "apt",
            // TODO!
            _ => "",
        }
    }
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bk_ = self.as_ref();
        write!(f, "{bk_}")
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
