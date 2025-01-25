pub mod backend_reg;
pub mod tool_reg;

use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

pub trait Reg<T> {
    fn load() -> Result<T>;
}
