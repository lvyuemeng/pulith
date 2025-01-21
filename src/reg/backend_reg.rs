use crate::{
    backend::BackendType,
    reg::{Cache, Reg},
};
use anyhow::Result;
use once_cell::sync::Lazy;

static BackendReg: Cache = Lazy::new(|| BackendRegAPI::load()?);

#[derive(Default, Debug)]
pub struct BackendRegConfig {}

pub struct BackendRegAPI;

impl Reg for BackendRegAPI {
    type Key = BackendType;
    type Val = ();

    fn load() -> Result<Cache<Self::Key, Self::Val>> {
        todo!()
    }
}
