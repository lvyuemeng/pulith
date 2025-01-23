use anyhow::{Context, Result};
use home::home_dir;
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default)]
pub struct PulithEnv {
    home: PathBuf,
    pwd: PathBuf,
    store: Store,
}

impl PulithEnv {
    pub fn new() -> Result<Self> {
        let home = home_dir().context("Failed to get home directory")?;
        let pwd = env::current_dir().context("Failed to get current directory")?;
        let root = env::var("PULITH_ROOT")
            .map(PathBuf::from)
            .unwrap_or(home.join(".pulith"));

        Ok(Self {
            home,
            pwd,
            store: Store::from(&root),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct Store {
    root: PathBuf,
    bin: PathBuf,
    cache: PathBuf,
    temp: PathBuf,
}

impl Store {
    pub fn from(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            bin: root.join("bin"),
            cache: root.join("cache"),
            temp: root.join("temp"),
        }
    }
}
