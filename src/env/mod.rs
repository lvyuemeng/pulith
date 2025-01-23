pub mod exec;
pub mod pulith;

use anyhow::{Result, bail};
use once_cell::sync::Lazy;
use query_shell::Shell;
use sysinfo::System;

#[derive(Debug, Clone, Copy)]
enum OS {
    Windows,
    Macos,
    Linux(Linux),
    Unknown,
}

#[derive(Debug, Clone, Copy)]
enum Linux {
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

#[derive(Debug, Clone, Copy)]
enum Arch {
    X86,
    X86_64,
    ARM,
    ARM64,
    Unknown,
}

impl From<&str> for OS {
    fn from(s: &str) -> Self {
        match s {
            "Windows" => OS::Windows,
            "macOS" => OS::Macos,
            _ => OS::Linux(Linux::from(s)),
        }
    }
}

impl From<&str> for Linux {
    fn from(s: &str) -> Linux {
        match s {
            "Debian GNU/Linux" => Linux::Debian,
            "Ubuntu" => Linux::Ubuntu,
            "Linux Mint" => Linux::LinuxMint,
            "Fedora" => Linux::Fedora,
            "Red Hat Enterprise Linux" => Linux::RedHatEnterpriseLinux,
            "CentOS Linux" => Linux::CentOS,
            "Arch Linux" => Linux::ArchLinux,
            "Manjaro Linux" => Linux::Manjaro,
            "openSUSE Leap" | "openSUSE Tumbleweed" => Linux::OpenSUSE,
            "Gentoo" => Linux::Gentoo,
            "Alpine Linux" => Linux::AlpineLinux,
            "Kali Linux" => Linux::KaliLinux,
            _ => Linux::Unknown,
        }
    }
}

impl From<&str> for Arch {
    fn from(s: &str) -> Self {
        match s {
            "i386" | "i686" => Arch::X86,
            "x86_64" => Arch::X86_64,
            "arm" | "armv7l" => Arch::ARM,
            "aarch64" => Arch::ARM64,
            _ => Arch::Unknown,
        }
    }
}

static SYSTEM_INFO: Lazy<SystemInfo> = Lazy::new(|| SystemInfo::load());

#[derive(Debug)]
pub struct SystemInfo {
    os: OS,
    arch: Arch,
}

impl SystemInfo {
    fn load() -> Self {
        let arch = System::cpu_arch().as_str().into();
        let Some(os) = System::name().map(|s| OS::from(s.as_str())) else {
            return Self {
                os: OS::Unknown,
                arch,
            };
        };

        Self { os, arch }
    }

    pub fn shell_exec() -> Result<&'static str> {
        match query_shell::get_shell() {
            Ok(s) => match s {
                Shell::Bash => Ok("bash"),
                Shell::Elvish => Ok("elvish"),
                Shell::Fish => Ok("fish"),
                Shell::Ion => Ok("ion"),
                Shell::Nushell => Ok("nu"),
                Shell::Powershell => Ok("pwsh"),
                Shell::Xonsh => Ok("xonsh"),
                Shell::Zsh => Ok("zsh"),
                _ => bail!("unsupported shell"),
            },
            Err(_) => bail!("failed to get shell"),
        }
    }
}
