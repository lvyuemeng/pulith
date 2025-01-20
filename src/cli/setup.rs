use crate::cli::app::App;
use anyhow::{Context, Result};
use clap::{Args, CommandFactory};
use clap_complete::{Shell, generate};

#[derive(Args, Clone, Debug)]
pub struct Setup {
    #[arg(long, help = "Shell to setup")]
    shell: Option<query_shell::Shell>,
}

pub fn setup(arg: Setup) -> Result<()> {
    // path logic

    let shell = match arg.shell {
        Some(s) => s,
        None => query_shell::get_shell().with_context(|| "Failed to get shell")?,
    };

    // completion

    let mut c = App::command();
    let mut stdio = std::io::stdout();
    let clap_shell = match shell {
        query_shell::Shell::Bash => Shell::Bash,
        query_shell::Shell::Elvish => Shell::Elvish,
        query_shell::Shell::Fish => Shell::Fish,
        query_shell::Shell::Powershell => Shell::PowerShell,
        query_shell::Shell::Zsh => Shell::Zsh,
        _ => {
            // TODO
        }
    };

    generate(clap_shell, &mut c, "pulith", &mut stdio);

    Ok(())
}
