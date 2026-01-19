//! Shell detection and helpers.

pub use query_shell::Shell as QueryShell;
use std::path::PathBuf;

/// Shell types supported by the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Pwsh,
    Cmd,
    Nushell,
    Elvish,
    Ion,
    Xonsh,
    Unknown,
}

fn from_query_shell(qs: QueryShell) -> Shell {
    match qs {
        QueryShell::Bash => Shell::Bash,
        QueryShell::Zsh => Shell::Zsh,
        QueryShell::Fish => Shell::Fish,
        QueryShell::Powershell => Shell::Powershell,
        QueryShell::Nushell => Shell::Nushell,
        QueryShell::Elvish => Shell::Elvish,
        QueryShell::Ion => Shell::Ion,
        QueryShell::Xonsh => Shell::Xonsh,
        _ => Shell::Unknown,
    }
}

/// Detect the current shell.
///
/// Returns `None` if detection fails.
pub fn detect() -> Option<Shell> {
    query_shell::get_shell().ok().map(from_query_shell)
}

/// Get the executable path for a shell.
///
/// Returns `None` if the shell is unknown or not available.
pub fn executable(shell: Shell) -> Option<&'static str> {
    match shell {
        Shell::Bash => Some("bash"),
        Shell::Zsh => Some("zsh"),
        Shell::Fish => Some("fish"),
        Shell::Powershell => Some("powershell"),
        Shell::Pwsh => Some("pwsh"),
        Shell::Cmd => Some("cmd.exe"),
        Shell::Nushell => Some("nu"),
        Shell::Elvish => Some("elvish"),
        Shell::Ion => Some("ion"),
        Shell::Xonsh => Some("xonsh"),
        Shell::Unknown => None,
    }
}

/// Get the current shell's config directory.
pub fn config_dir(shell: Shell) -> Option<PathBuf> {
    use std::env;

    let (base, name) = match shell {
        Shell::Bash => ("xdg", "bash"),
        Shell::Zsh => ("xdg", "zsh"),
        Shell::Fish => ("xdg", "fish"),
        Shell::Nushell => ("xdg", "nushell"),
        Shell::Elvish => ("xdg", "elvish"),
        Shell::Ion => ("xdg", "ion"),
        Shell::Xonsh => ("xdg", "xonsh"),

        Shell::Powershell | Shell::Pwsh => ("powershell", "PowerShell"),
        Shell::Cmd | Shell::Unknown => return None,
    };

    match base {
        "powershell" => home::home_dir().map(|home| home.join("Documents").join(name)),
        "xdg" => {
            if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
                Some(PathBuf::from(xdg).join(name))
            } else {
                home::home_dir().map(|h| h.join(".config").join(name))
            }
        }
        _ => None,
    }
}
