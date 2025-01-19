use anyhow::Result;
use once_cell::sync::Lazy;

static ToolReg: Cache<> = Lazy::new(|| ToolRegAPI::get_or_init()?);

#[derive(Default, Debug)]
pub struct ToolRegConfig {}

pub struct ToolRegAPI;

impl ToolRegAPI {
    type ctx = Cache;
}
