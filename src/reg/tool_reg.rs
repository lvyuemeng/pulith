use anyhow::Result;
use once_cell::sync::Lazy;

static ToolReg: Cache<T> = Lazy::new(|| ToolRegAPI::get_or_init()?);

#[derive(Default, Debug)]
pub struct ToolRegConfig {}

pub struct ToolRegAPI;

impl ToolRegAPI {
    type ctx = Cache;

    pub fn get_or_init() -> Result {}
}
