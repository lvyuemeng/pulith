use crate::cli::setup::SetupArg;
use crate::cli::tool::{add,rm,search,up,use_ver};
use clap::{Parser, Subcommand};

use super::tool::ls;

#[derive(Clone, Debug, Parser)]
#[command(name="Pulith",version=env!("CARGO_PKG_VERSION"),about,long_about=None,propagate_version=true)]
pub struct App {
    pub cmd: Commands,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "up", name = "upgrade")]
    Upgrade(Upgrade),
    #[command(alias = "cfg", name = "config")]
    Config(Config),
    #[command(alias = "s", name = "setup", about = "Setup completion and env var")]
    SetUp(SetupArg),

    // Backend
    #[command(alias = "i", name = "info")]
    Info(Info),
    #[command(alias = "ls", name = "list")]
    List(List),
    #[command(alias = "u", name = "update")]
    Update(Update),
    #[command(alias = "rm", name = "remove")]
    Remove(Remove),
    #[command(alias = "a", name = "add")]
    Add(Add),

    //Tool
    #[command(alias = "t", name = "tool")]
    Tool(ToolCommands),
}

#[derive(Clone, Debug, Subcommand)]
pub enum ToolCommands {
    #[command(alias = "s", name = "search")]
    Search(search::SearchArg),
    #[command(alias = "a", name = "add")]
    Add(add::AddArg),
    #[command(alias = "u", name = "use")]
    Use(use_ver::UseArg),
    #[command(alias = "rm", name = "remove")]
    Remove(rm::RmArg),
    #[command(alias = "ls", name = "list")]
    List(ls::ListArg),
    #[command(alias = "u", name = "update")]
    Update(up::UpdateArg),
}
