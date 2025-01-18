use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum BackendType {
    Unknown,
}

impl BackendType {
    fn from_str(s: &str) -> Self {
        match s {
            _ => BackendType::Unknown,
        }
    }

    fn is(name: BackendType) -> Option<Box<dyn Backend>> {
        match name {
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackendTool {
    bk: BackendType,
    name: String,
}

fn parse_tool(s: &str) -> Result<BackendTool> {
    let (backend_str, tool) = s.split_once(":")?;
    let bk = BackendType::from_str(backend_str);
    let name = tool.to_string();
    Ok(BackendTool { bk, name })
}

