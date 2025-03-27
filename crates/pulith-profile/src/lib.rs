pub mod profile;
pub mod alias;
pub mod flag;
pub mod ctx;

use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    process::Command,
};

use figment::{
    Figment,
    providers::{self, Format},
};
use serde::Deserialize;
use tera::{Context, Tera};
use thiserror::Error;

use crate::profile::Profile;

pub struct FrameApi {
    root: PathBuf,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
    #[error(transparent)]
    FigmentError(#[from] figment::Error),
    #[error(transparent)]
    TeraError(#[from] tera::Error),
}

#[derive(Debug,Error)]
pub enum AliasError {

}

impl FrameApi {
    // root
    // - /pulith
    //   - /data
    //   - /script
    //   profile.toml
    //   config.toml
    const PULITH_ROOT: &str = "/pulith";
    const DATA_ROOT: &str = "/pulith/data";
    const SCRIPT_ROOT: &str = "/pulith/script";

    const PROFILES_PREFIX: &str = "profile";
    const CONFIG_PREFIX: &str = "config";

    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn get_config<T: for<'a> Deserialize<'a>>(&self) -> Result<T, Error> {
        let config_path = self.root.join(Self::PULITH_ROOT).join(Self::CONFIG_PREFIX);
        let fig = Figment::new().merge(providers::Toml::file(config_path));
        fig.extract().map_err(|e| Error::FigmentError(e))
    }

    pub fn get_profile(&self) -> Result<Profile, Error> {
        let profile_path = self
            .root
            .join(Self::PULITH_ROOT)
            .join(Self::PROFILES_PREFIX);
        let profile_path = profile_path.to_str().unwrap();
        // tera render
        let tera = match Tera::new(profile_path) {
            Ok(tera) => tera,
            Err(e) => Err(Error::TeraError(e))?,
        };
        let p = tera.render("profile", &self.get_data())?;

        // parse toml
        let p: Profile = toml::from_str(&p)?;
        Ok(p)
    }

    pub fn get_data(&self) -> Context {
        // read files from DATA_ROOT
        // Add prefix as the files name s.t. .{file_name}.{data_name} for insertion
        // insert context s.t. {new_name} = {data}
        let mut ctx = Context::new();
        let data_path = self.root.join(Self::PULITH_ROOT).join(Self::DATA_ROOT);
        for entry in read_dir(data_path).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_str().unwrap().to_string();
            let data: toml::Value =
                toml::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
            for (k, v) in data.as_table().unwrap().iter() {
                ctx.insert(&format!(".{}.{}", name, k), v);
            }
        }

        ctx
    }

    pub fn get_cmd_script(&self) -> Vec<(String, String)> {
        // read profile alias.script from profile
        // read file names from SCRIPT_ROOT
        // check existence of script by profile
        // exec script and wait for completion
        let script_path = self.root.join(Self::PULITH_ROOT).join(Self::SCRIPT_ROOT);
        let mut scripts = Vec::new();
        for entry in read_dir(script_path).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_str().unwrap().to_string();
            scripts.push(name);
        }
        let profile = self.get_profile().unwrap();
        let profile_scripts = profile.command.script;
        let profile_scripts = profile_scripts
            .iter()
            .filter_map(|(k, v)| {
                if scripts.contains(&v) {
                    Some((k.clone(), v.clone()))
                } else {
                    None
                }
            })
            .collect();
        profile_scripts
    }

    pub fn script_command(&self) -> Vec<clap::Command> {
        let scripts = self.get_cmd_script();
        let args = scripts
            .into_iter()
            .map(|(k, v)| {
                let arg = clap::Command::new(k).about(format!("run script {}", v));
                arg
            })
            .collect();

        args
    }

    pub fn get_cmd_alias(&self) -> Vec<(String, String)> {
        let profile = self.get_profile().unwrap();
        profile.command.t.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}
