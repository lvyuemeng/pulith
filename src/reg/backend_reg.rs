use anyhow::Result;
use once_cell::sync::Lazy;

static BackendReg: Cache = Lazy::new(|| BackendRegAPI::get_or_init()?);

#[derive(Default, Debug)]
pub struct BackendRegConfig {}

pub struct BackendRegAPI;

impl BackendRegAPI {
    type ctx = Cache;
    pub fn get_or_init() -> Result {}
}
