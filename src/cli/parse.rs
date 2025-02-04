use anyhow::Context;

use crate::{backend::BackendType, utils::ver::VersionKind};

#[derive(Debug, Clone)]
pub struct Descriptor {
    bk: Option<BackendType>,
    name: Option<String>,
    ver_key: Option<VersionKind>,
}

impl TryFrom<&str> for Descriptor {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut s = s.trim_end().split(':');

        let bk = s.next().map(BackendType::from_str);
        let name = s.next().map(|s| s.to_string());
        let ver_key = s.next().map(VersionKind::try_from).transpose()?;

        Ok(Descriptor { bk, name, ver_key })
    }
}

impl Descriptor {
    pub fn only_bk(self) -> Option<BackendType> {
        if self.name.is_none() && self.ver_key.is_none() {
            self.bk
        } else {
            None
        }
    }
    pub fn only_tool(self) -> Option<(String, VersionKind)> {
        if let (Some(name), Some(ver_key)) = (self.name, self.ver_key) && self.bk.is_none() {
            Some((name, ver_key))
        } else {
            None
        }
    }
}
