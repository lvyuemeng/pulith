pub mod ver;

use anyhow::{Context, Result};
use ver::VersionKey;

use crate::backend::BackendType;

#[derive(Debug, Clone)]
pub struct BackendTool {
    bk: Option<BackendType>,
    name: String,
    ver_key: Option<VersionKey>,
}

impl TryFrom<&str> for BackendTool {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut s = s.trim_end().split(':');

        let bk = s.next().map(BackendType::from_str);
        let name = s.next().with_context(|| "missing tool name")?;
        let ver_key = s.next().map(VersionKey::try_from).transpose()?;

        Ok({
            BackendTool {
                bk,
                name: name.to_string(),
                ver_key,
            }
        })
    }
}
