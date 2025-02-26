use crate::backend::{
    Add, Backend, BackendType, Metadata, Snap,
    reg::backend_reg::{BackendRegAPI, BackendRegLoader},
};

use anyhow::Result;
use std::{
    path::PathBuf,
};

// Native Package Manager(with out Snap)
#[derive(Debug, Clone)]
#[cfg_attr(target_os = "windows", allow(dead_code))]
pub struct Winget(PathBuf);

impl Winget {
    pub fn new() -> Result<Self> {
        let path = which::which("winget")?;
        Ok(Winget(path))
    }
}

impl Backend for Winget {
    fn metadata(&self) -> Metadata {
		Metadata::new(
			"winget",
			"https://github.com/microsoft/winget-cli",
			"Windows Native Package Manager",
		)
	}
}

impl Add for Winget {
    type Ctx = String;
    fn add(&self, ctx: Self::Ctx) -> anyhow::Result<()> {
		
	}
}
