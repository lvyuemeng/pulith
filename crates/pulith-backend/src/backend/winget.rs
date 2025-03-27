use crate::backend::{Add, Backend, Metadata};
use crate::package::Package;

use std::path::PathBuf;
use std::process::Command;

// Native Package Manager(with out Snap)
#[derive(Debug)]
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
    type Ctx = Package;
    fn add(&self, ctx: Self::Ctx) -> Result<()> {
        let mut cmd = Command::new(&self.0);
        cmd.args(["add", &ctx.name()]);
        if let Some(ver) = ctx.ver() {
            cmd.args(["-v", &format!("{ver}")]);
        }
        cmd.output()
	}
}
