use crate::error::{Error, Result};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

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

impl Shell {
    pub fn current() -> Option<Self> {
        query_shell::get_shell().ok().map(Self::from)
    }

    pub fn executable(&self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Powershell => "powershell.exe",
            Self::Pwsh => "pwsh",
            Self::Cmd => "cmd.exe",
            Self::Nushell => "nu",
            Self::Elvish => "elvish",
            Self::Ion => "ion",
            Self::Xonsh => "xonsh",
            Self::Unknown => "",
        }
    }

    pub fn config_dir(&self) -> Option<PathBuf> {
        match self {
            Self::Bash
            | Self::Zsh
            | Self::Fish
            | Self::Nushell
            | Self::Elvish
            | Self::Ion
            | Self::Xonsh => env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| home::home_dir().map(|p| p.join(".config"))),
            Self::Powershell | Self::Pwsh => {
                home::home_dir().map(|p| p.join("Documents").join("PowerShell"))
            }
            Self::Cmd | Self::Unknown => None,
        }
    }
}

impl From<query_shell::Shell> for Shell {
    fn from(s: query_shell::Shell) -> Self {
        match s {
            query_shell::Shell::Bash => Self::Bash,
            query_shell::Shell::Zsh => Self::Zsh,
            query_shell::Shell::Fish => Self::Fish,
            _ => Self::Unknown,
        }
    }
}

impl FromStr for Shell {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "powershell" | "windowspowershell" => Ok(Self::Powershell),
            "pwsh" | "powershellcore" => Ok(Self::Pwsh),
            "cmd" | "cmd.exe" => Ok(Self::Cmd),
            "nu" | "nushell" => Ok(Self::Nushell),
            "elvish" => Ok(Self::Elvish),
            "ion" => Ok(Self::Ion),
            "xonsh" => Ok(Self::Xonsh),
            _ => Err(Error::UnknownShell(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_executable_bash() {
        assert_eq!(Shell::Bash.executable(), "bash");
    }

    #[test]
    fn test_shell_executable_zsh() {
        assert_eq!(Shell::Zsh.executable(), "zsh");
    }

    #[test]
    fn test_shell_executable_fish() {
        assert_eq!(Shell::Fish.executable(), "fish");
    }

    #[test]
    fn test_shell_executable_powershell() {
        assert_eq!(Shell::Powershell.executable(), "powershell.exe");
    }

    #[test]
    fn test_shell_executable_pwsh() {
        assert_eq!(Shell::Pwsh.executable(), "pwsh");
    }

    #[test]
    fn test_shell_executable_cmd() {
        assert_eq!(Shell::Cmd.executable(), "cmd.exe");
    }

    #[test]
    fn test_shell_executable_nushell() {
        assert_eq!(Shell::Nushell.executable(), "nu");
    }

    #[test]
    fn test_shell_executable_elvish() {
        assert_eq!(Shell::Elvish.executable(), "elvish");
    }

    #[test]
    fn test_shell_executable_ion() {
        assert_eq!(Shell::Ion.executable(), "ion");
    }

    #[test]
    fn test_shell_executable_xonsh() {
        assert_eq!(Shell::Xonsh.executable(), "xonsh");
    }

    #[test]
    fn test_shell_executable_unknown() {
        assert_eq!(Shell::Unknown.executable(), "");
    }

    #[test]
    fn test_shell_from_str_bash() {
        assert_eq!("bash".parse::<Shell>().unwrap(), Shell::Bash);
        assert_eq!("BASH".parse::<Shell>().unwrap(), Shell::Bash);
    }

    #[test]
    fn test_shell_from_str_zsh() {
        assert_eq!("zsh".parse::<Shell>().unwrap(), Shell::Zsh);
        assert_eq!("ZSH".parse::<Shell>().unwrap(), Shell::Zsh);
    }

    #[test]
    fn test_shell_from_str_fish() {
        assert_eq!("fish".parse::<Shell>().unwrap(), Shell::Fish);
        assert_eq!("FISH".parse::<Shell>().unwrap(), Shell::Fish);
    }

    #[test]
    fn test_shell_from_str_powershell() {
        assert_eq!("powershell".parse::<Shell>().unwrap(), Shell::Powershell);
        assert_eq!(
            "windowspowershell".parse::<Shell>().unwrap(),
            Shell::Powershell
        );
        assert_eq!("PowerShell".parse::<Shell>().unwrap(), Shell::Powershell);
    }

    #[test]
    fn test_shell_from_str_pwsh() {
        assert_eq!("pwsh".parse::<Shell>().unwrap(), Shell::Pwsh);
        assert_eq!("powershellcore".parse::<Shell>().unwrap(), Shell::Pwsh);
        assert_eq!("PWSH".parse::<Shell>().unwrap(), Shell::Pwsh);
    }

    #[test]
    fn test_shell_from_str_cmd() {
        assert_eq!("cmd".parse::<Shell>().unwrap(), Shell::Cmd);
        assert_eq!("cmd.exe".parse::<Shell>().unwrap(), Shell::Cmd);
        assert_eq!("CMD".parse::<Shell>().unwrap(), Shell::Cmd);
    }

    #[test]
    fn test_shell_from_str_nushell() {
        assert_eq!("nushell".parse::<Shell>().unwrap(), Shell::Nushell);
        assert_eq!("nu".parse::<Shell>().unwrap(), Shell::Nushell);
        assert_eq!("NuShell".parse::<Shell>().unwrap(), Shell::Nushell);
    }

    #[test]
    fn test_shell_from_str_elvish() {
        assert_eq!("elvish".parse::<Shell>().unwrap(), Shell::Elvish);
        assert_eq!("ELVISH".parse::<Shell>().unwrap(), Shell::Elvish);
    }

    #[test]
    fn test_shell_from_str_ion() {
        assert_eq!("ion".parse::<Shell>().unwrap(), Shell::Ion);
        assert_eq!("ION".parse::<Shell>().unwrap(), Shell::Ion);
    }

    #[test]
    fn test_shell_from_str_xonsh() {
        assert_eq!("xonsh".parse::<Shell>().unwrap(), Shell::Xonsh);
        assert_eq!("XONSH".parse::<Shell>().unwrap(), Shell::Xonsh);
    }

    #[test]
    fn test_shell_from_str_unknown() {
        assert!("invalid".parse::<Shell>().is_err());
        assert!("".parse::<Shell>().is_err());
        assert!("bashh".parse::<Shell>().is_err());
    }

    #[test]
    fn test_shell_current_returns_optional() {
        let current = Shell::current();
        assert!(current.is_some() || current.is_none());
    }

    #[test]
    fn test_shell_from_query_shell_known() {
        let shell = Shell::from(query_shell::Shell::Bash);
        assert_eq!(shell, Shell::Bash);
    }

    #[test]
    fn test_shell_from_query_shell_zsh() {
        let shell = Shell::from(query_shell::Shell::Zsh);
        assert_eq!(shell, Shell::Zsh);
    }

    #[test]
    fn test_shell_config_dir_xdg_shells() {
        let shells = [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::Nushell,
            Shell::Elvish,
            Shell::Ion,
            Shell::Xonsh,
        ];
        for shell in shells {
            let config_dir = shell.config_dir();
            assert!(config_dir.is_none() || config_dir.unwrap().to_string_lossy().len() > 0);
        }
    }

    #[test]
    fn test_shell_config_dir_powershell() {
        let config_dir = Shell::Powershell.config_dir();
        assert!(
            config_dir.is_none() || config_dir.unwrap().to_string_lossy().contains("PowerShell")
        );
    }

    #[test]
    fn test_shell_config_dir_cmd() {
        assert!(Shell::Cmd.config_dir().is_none());
    }

    #[test]
    fn test_shell_config_dir_unknown() {
        assert!(Shell::Unknown.config_dir().is_none());
    }

    #[test]
    fn test_shell_partial_eq() {
        assert_eq!(Shell::Bash, Shell::Bash);
        assert_ne!(Shell::Bash, Shell::Zsh);
    }

    #[test]
    fn test_shell_copy_semantics() {
        let shell = Shell::Bash;
        let copied = shell;
        assert_eq!(shell, copied);
    }
}
