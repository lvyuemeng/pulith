pub mod backend_reg;
pub mod tool_reg;

use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

pub type Cache<K, V> = Lazy<BTreeMap<K, V>>;

pub trait Reg {
    type Key;
    type Val;
    fn load() -> Result<Cache<Self::Key, Self::Val>>;
}
