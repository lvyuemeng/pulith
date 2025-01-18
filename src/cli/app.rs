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
    Search(ToolSearch),
    #[command(alias = "a", name = "add")]
    Add(ToolAdd),
    #[command(alias = "u", name = "use")]
    Use(ToolUse),
    #[command(alias = "rm", name = "remove")]
    Remove(ToolRemove),
    #[command(alias = "ls", name = "list")]
    List(ToolList),
    #[command(alias = "u", name = "update")]
    Update(ToolUpdate),
}
