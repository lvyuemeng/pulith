use crate::backend::{Add, Backend, Snap,Metadata,reg::backend_reg::{BackendRegAPI,BackendRegLoader},BackendType};

use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::bail;

#[derive(Debug,Clone)]
#[cfg_attr(target_os="windows",allow(dead_code))]
pub struct Winget<'a>(&'a Snap);

impl Backend for Winget<'_> {
	pub fn new(reg:&BackendRegLoader) -> Result<Self> {
		if !cfg!(target_os = "windows") {
			bail!("Winget is only available on Windows");
		}

		if let Some(snap) = BackendRegAPI::get_snap(reg, &BackendType::Winget)  {
			return Ok(Winget(&snap));
		} 
		let path = which::which("winget")?;
		let systime = SystemTime::now().duration_since(UNIX_EPOCH)?;
		
	}
	
	fn exec(&self, args: &[&str]) -> anyhow::Result<String> {
			todo!()
		}
	
	fn metadata(&self) -> Metadata {
			todo!()
		}
}

impl Add for Winget {
	type Ctx = String;
	fn add(&self, ctx: Self::Ctx) -> anyhow::Result<()> {

	}
}