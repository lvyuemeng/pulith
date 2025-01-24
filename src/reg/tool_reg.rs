use crate::reg::{Cache, Reg};
use anyhow::Result;
use once_cell::sync::Lazy;

static ToolReg: Cache<> = Lazy::new(|| ToolRegAPI::load()?);

#[derive(Default, Debug)]
pub struct ToolRegConfig {}

pub struct ToolRegAPI;

impl Reg for ToolRegAPI {
    type Key = ();
    type Val = ();

    fn load() -> Result<Cache<Self::Key, Self::Val>> {
        todo!()
    }
}
