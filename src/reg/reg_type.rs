use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

type Cache<K, V> = Lazy<BTreeMap<K, V>>;

trait Reg<K, V> {
    type Config;
    type Out;
    fn get_or_init() -> Result<Cache<K, V>>;
    fn list() -> Self::Out;
}
