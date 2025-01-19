pub mod backend_reg;
pub mod tool_reg;

use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

type Cache<K, V> = Lazy<BTreeMap<K, V>>;

pub trait Reg<K, V> {
    type Config;
    type Out;
    fn load() -> Result<Cache<K, V>>;
    fn list() -> Self::Out;
}
