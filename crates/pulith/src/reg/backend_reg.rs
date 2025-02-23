use crate::env::PulithEnv;

use std::{collections::HashMap, path::PathBuf};
use anyhow::Result;
use once_cell::sync::Lazy;

pub static BACKEND_REG: Lazy<BackendReg> = Lazy::new(|| BackendReg::load()?);

pub struct BackendRegAPI;

impl BackendRegAPI {
}

type BackendReg = Reg<HashMap<BackendType, Snap>>;

impl Cache for BackendReg {
    fn locate() -> Result<PathBuf> {
        Ok(PulithEnv::new()?.store().root().join("backend.reg.lock"))
    }

    fn load() -> Result<Self>;
    fn save(&self) -> Result<()>;
}
