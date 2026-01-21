//! Platform utilities (OS, architecture, path, shell).

use once_cell::sync::Lazy;
use query_shell::Shell as QueryShell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OS {
    Windows,
    Macos,
    Linux(Distro),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distro {
    Debian,
    Ubuntu,
    LinuxMint,
    Fedora,
    RedHatEnterpriseLinux,
    CentOS,
    ArchLinux,
    Manjaro,
    OpenSUSE,
    Gentoo,
    AlpineLinux,
    KaliLinux,
    Unknown,
}

static SYSTEM_INFO: Lazy<SystemInfo> = Lazy::new(SystemInfo::load);

struct SystemInfo {
    os:     OS,
    distro: Option<Distro>,
}

impl SystemInfo {
    fn load() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os = match System::name().as_deref() {
            Some("Windows") => OS::Windows,
            Some("macOS") => OS::Macos,
            Some(name) if name.starts_with("Linux") => {
                let distro = detect_distro();
                OS::Linux(distro)
            }
            _ => OS::Unknown,
        };

        let distro = if let OS::Linux(d) = os { Some(d) } else { None };

        Self { os, distro }
    }
}

fn detect_distro() -> Distro {
    let content = fs::read_to_string("/etc/os-release").ok();
    let content = content.as_ref().map(|s| s.as_str()).unwrap_or("");

    for line in content.lines() {
        if line.starts_with("ID=") {
            let distro = line.trim_start_matches("ID=").trim_matches('"');
            return match distro {
                "debian" => Distro::Debian,
                "ubuntu" => Distro::Ubuntu,
                "linuxmint" => Distro::LinuxMint,
                "fedora" => Distro::Fedora,
                "rhel" | "redhat" | "centos" => {
                    if distro == "centos" {
                        Distro::CentOS
                    } else {
                        Distro::RedHatEnterpriseLinux
                    }
                }
                "arch" => Distro::ArchLinux,
                "manjaro" => Distro::Manjaro,
                "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" => Distro::OpenSUSE,
                "gentoo" => Distro::Gentoo,
                "alpine" => Distro::AlpineLinux,
                "kali" => Distro::KaliLinux,
                _ => Distro::Unknown,
            };
        }
    }

    Distro::Unknown
}

pub fn detect() -> OS { SYSTEM_INFO.os }

pub fn distro() -> Distro { SYSTEM_INFO.distro.unwrap_or(Distro::Unknown) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X86_64,
    ARM,
    ARM64,
    Unknown,
}

pub fn detect_arch() -> Arch {
    let cpu_arch = sysinfo::System::cpu_arch();
    let arch_str = cpu_arch.as_str();

    match arch_str {
        "i386" | "i686" => Arch::X86,
        "x86_64" => Arch::X86_64,
        "arm" | "armv7l" => Arch::ARM,
        "aarch64" | "arm64" => Arch::ARM64,
        _ => Arch::Unknown,
    }
}

pub fn target_triple(arch: Arch) -> &'static str {
    match arch {
        Arch::X86 => "i686-unknown-linux-gnu",
        Arch::X86_64 => "x86_64-unknown-linux-gnu",
        Arch::ARM => "arm-unknown-linux-gnueabihf",
        Arch::ARM64 => "aarch64-unknown-linux-gnu",
        Arch::Unknown => "unknown-unknown-unknown",
    }
}

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

pub fn detect_shell() -> Option<Shell> { query_shell::get_shell().ok().map(from_query_shell) }

pub fn shell_executable(shell: Shell) -> Option<&'static str> {
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

pub fn shell_config_dir(shell: Shell) -> Option<PathBuf> {
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

pub fn user_home() -> Option<PathBuf> { home::home_dir() }

pub fn user_config() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        user_home().map(|p| p.join("Library/Application Support"))
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| user_home().map(|p| p.join(".config")))
    }
}

pub fn user_data() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        user_home().map(|p| p.join("Library/Application Support"))
    } else {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| user_home().map(|p| p.join(".local/share")))
    }
}

pub fn user_temp() -> PathBuf { env::temp_dir() }

pub fn path_env() -> Option<Vec<PathBuf>> {
    let path_var = if cfg!(target_os = "windows") {
        "Path"
    } else {
        "PATH"
    };
    env::var_os(path_var).map(|v| env::split_paths(&v).collect())
}

pub fn prepend_path(path: &Path) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = path_env()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p != path)
        .collect();

    entries.insert(0, path.to_path_buf());

    let new_path = env::join_paths(entries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    unsafe { env::set_var("PATH", new_path) };
    Ok(())
}

pub fn remove_path(path: &Path) -> std::io::Result<()> {
    let entries: Vec<PathBuf> = path_env()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p != path)
        .collect();

    let new_path = env::join_paths(entries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    unsafe { env::set_var("PATH", new_path) };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Arch, OS, Shell};

    #[test]
    fn test_os_detection() {
        let os = super::detect();
        match os {
            OS::Windows | OS::Macos | OS::Linux(_) | OS::Unknown => {}
        }
    }

    #[test]
    fn test_arch_detection() {
        let arch = super::detect_arch();
        match arch {
            Arch::X86 | Arch::X86_64 | Arch::ARM | Arch::ARM64 | Arch::Unknown => {}
        }
    }

    #[test]
    fn test_shell_executable() {
        assert_eq!(super::shell_executable(Shell::Bash), Some("bash"));
        assert_eq!(super::shell_executable(Shell::Unknown), None);
    }

    #[test]
    fn test_user_home() {
        let home = super::user_home();
        if let Some(p) = home {
            assert!(p.exists() || p.to_string_lossy().len() > 0);
        }
    }

    #[test]
    fn test_temp_dir() {
        let temp = super::user_temp();
        assert!(temp.exists() || temp.to_string_lossy().len() > 0);
    }
}
