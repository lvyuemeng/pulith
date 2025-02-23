use clap::{Parser, Subcommand};

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
    #[command(alias = "s", name = "search")]
    Search(SearchArg),
    #[command(alias = "u", name = "use")]
    Use(UseArg),
}
