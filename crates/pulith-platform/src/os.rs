//! Operating system and distribution detection.

use once_cell::sync::Lazy;
use std::fs;
use sysinfo::System;

/// Operating system types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OS {
    Windows,
    Macos,
    Linux(Distro),
    Unknown,
}

/// Linux distribution types.
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
    os: OS,
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

/// Detect current operating system.
pub fn detect() -> OS {
    SYSTEM_INFO.os
}

/// Detect current Linux distribution.
/// Only valid if OS::Linux, returns Distro::Unknown otherwise.
pub fn distro() -> Distro {
    SYSTEM_INFO.distro.unwrap_or(Distro::Unknown)
}
