use std::{fmt, intrinsics::raw_eq};

use display_tree::{DisplayTree, format_tree};

use crate::{
    backend::{Backend, BackendType},
    reg::tool_reg::ToolRegAPI,
};

#[derive(Debug, clap::Args)]
#[clap(visible_alias = "list")]
pub struct Ls {}

impl Ls {
    pub fn run(self) {
        let tree = LsTree {
            nodes: BackendType::all()
                .filter_map(|bk| {
                    let bk = BackendStatus::from(bk);

                    if bk.installed && bk.bk.tools().is_some() {
                        return Some(Node {
                            bk,
                            tools: bk.bk.tools().unwrap().collect(),
                        });
                    }

                    None
                })
                .collect(),
        };

        format_tree!(tree);
    }
}

#[derive(DisplayTree)]
struct LsTree {
    #[tree]
    pub nodes: Vec<Node>,
}

#[derive(DisplayTree)]
struct Node {
    #[node_label]
    pub bk: BackendStatus,
    #[tree]
    pub tools: Option<Vec<String>>,
}

struct BackendStatus {
    pub bk: BackendType,
    pub installed: bool,
}

impl From<BackendType> for BackendStatus {
    fn from(value: BackendType) -> Self {
        Self {
            bk: value,
            installed: value.snap().is_some(),
        }
    }
}

impl fmt::Display for BackendStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.installed { "✓" } else { "✗" };

        write!(f, "{} {}", self.bk, status)
    }
}
