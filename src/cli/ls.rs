use display_tree::DisplayTree;

use crate::backend::BackendType;

#[derive(Debug, clap::Args)]
#[clap(visible_alias = "list")]
pub struct Ls {
    #[clap(long, short, conflicts_with = "backend")]
    tool: bool,
    #[clap(long, short)]
    backend: bool,
}

impl Ls {
    pub fn run() -> Result<()> {}
}

#[derive(DisplayTree)]
struct LsTree {
    #[node_label]
    bk:BackendType,
    #[tree]
    tool:
    
}

struct 