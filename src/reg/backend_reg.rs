use crate::{
    backend::{BackendType, Snap},
    reg::{Cache, Reg},
};
use anyhow::Result;
use once_cell::sync::Lazy;

static BACKEND_REG: Cache<BackendType, Snap> = Lazy::new(|| BackendRegAPI::load()?);

#[derive(Default, Debug)]
pub struct BackendRegConfig {}

pub struct BackendRegAPI;

impl Reg for BackendRegAPI {
    type Key = BackendType;
    type Val = Snap;

    fn load() -> Result<Cache<Self::Key, Self::Val>> {
        
    }
}

