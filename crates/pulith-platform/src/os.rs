use crate::error::{Error, Result};
use once_cell::sync::Lazy;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OS {
    Windows,
    MacOS,
    Linux,
    FreeBSD,
    Unknown,
}

impl OS {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "macos" => Self::MacOS,
            "linux" => Self::Linux,
            "freebsd" => Self::FreeBSD,
            _ => Self::Unknown,
        }
    }

    pub fn is_windows(&self) -> bool {
        matches!(self, Self::Windows)
    }
    pub fn is_macos(&self) -> bool {
        matches!(self, Self::MacOS)
    }
    pub fn is_linux(&self) -> bool {
        matches!(self, Self::Linux)
    }
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

impl FromStr for OS {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "windows" => Ok(Self::Windows),
            "macos" | "darwin" => Ok(Self::MacOS),
            "linux" => Ok(Self::Linux),
            "freebsd" => Ok(Self::FreeBSD),
            _ => Err(Error::UnknownOS(s.to_string())),
        }
    }
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
    None,
    Unknown,
}

impl Distro {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

impl FromStr for Distro {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "debian" => Ok(Self::Debian),
            "ubuntu" => Ok(Self::Ubuntu),
            "linuxmint" => Ok(Self::LinuxMint),
            "fedora" => Ok(Self::Fedora),
            "rhel" | "redhat" | "redhatenterprise" => Ok(Self::RedHatEnterpriseLinux),
            "centos" => Ok(Self::CentOS),
            "arch" | "archlinux" => Ok(Self::ArchLinux),
            "manjaro" => Ok(Self::Manjaro),
            "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" => Ok(Self::OpenSUSE),
            "gentoo" => Ok(Self::Gentoo),
            "alpine" => Ok(Self::AlpineLinux),
            "kali" => Ok(Self::KaliLinux),
            _ => Err(Error::UnknownDistro(s.to_string())),
        }
    }
}

static DISTRO: Lazy<Distro> = Lazy::new(|| {
    if !cfg!(target_os = "linux") {
        return Distro::None;
    }
    let content = std::fs::read_to_string("/etc/os-release").ok();
    let content = content.as_deref().unwrap_or("");
    for line in content.lines() {
        if line.starts_with("ID=") {
            let distro = line.trim_start_matches("ID=").trim_matches('"');
            return distro.parse().unwrap_or(Distro::Unknown);
        }
    }
    Distro::Unknown
});

pub fn detect_distro() -> Distro {
    *DISTRO
}

#[cfg(test)]
mod tests {
    #![allow(clippy::assertions_on_constants)]
    use super::*;

    #[test]
    fn test_os_current_matches_cfg() {
        let os = OS::current();
        match os {
            OS::Windows => assert!(cfg!(target_os = "windows")),
            OS::MacOS => assert!(cfg!(target_os = "macos")),
            OS::Linux => assert!(cfg!(target_os = "linux")),
            OS::FreeBSD => assert!(cfg!(target_os = "freebsd")),
            OS::Unknown => assert!(!cfg!(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "linux",
                target_os = "freebsd"
            ))),
        }
    }

    #[test]
    fn test_os_is_windows() {
        let os = OS::current();
        assert_eq!(os.is_windows(), cfg!(target_os = "windows"));
    }

    #[test]
    fn test_os_is_macos() {
        let os = OS::current();
        assert_eq!(os.is_macos(), cfg!(target_os = "macos"));
    }

    #[test]
    fn test_os_is_linux() {
        let os = OS::current();
        assert_eq!(os.is_linux(), cfg!(target_os = "linux"));
    }

    #[test]
    fn test_os_from_str_valid() {
        assert_eq!("windows".parse::<OS>().unwrap(), OS::Windows);
        assert_eq!("macos".parse::<OS>().unwrap(), OS::MacOS);
        assert_eq!("linux".parse::<OS>().unwrap(), OS::Linux);
        assert_eq!("freebsd".parse::<OS>().unwrap(), OS::FreeBSD);
    }

    #[test]
    fn test_os_from_str_case_insensitive() {
        assert_eq!("WINDOWS".parse::<OS>().unwrap(), OS::Windows);
        assert_eq!("MacOS".parse::<OS>().unwrap(), OS::MacOS);
        assert_eq!("LINUX".parse::<OS>().unwrap(), OS::Linux);
    }

    #[test]
    fn test_os_from_str_invalid() {
        assert!("invalid".parse::<OS>().is_err());
        assert!("".parse::<OS>().is_err());
        assert!("win".parse::<OS>().is_err());
    }

    #[test]
    fn test_distro_from_str_valid() {
        assert_eq!("ubuntu".parse::<Distro>().unwrap(), Distro::Ubuntu);
        assert_eq!("debian".parse::<Distro>().unwrap(), Distro::Debian);
        assert_eq!("arch".parse::<Distro>().unwrap(), Distro::ArchLinux);
        assert_eq!("fedora".parse::<Distro>().unwrap(), Distro::Fedora);
        assert_eq!(
            "rhel".parse::<Distro>().unwrap(),
            Distro::RedHatEnterpriseLinux
        );
        assert_eq!(
            "redhat".parse::<Distro>().unwrap(),
            Distro::RedHatEnterpriseLinux
        );
        assert_eq!("centos".parse::<Distro>().unwrap(), Distro::CentOS);
        assert_eq!("alpine".parse::<Distro>().unwrap(), Distro::AlpineLinux);
        assert_eq!("gentoo".parse::<Distro>().unwrap(), Distro::Gentoo);
    }

    #[test]
    fn test_distro_from_str_variant_names() {
        assert_eq!("archlinux".parse::<Distro>().unwrap(), Distro::ArchLinux);
        assert_eq!("linuxmint".parse::<Distro>().unwrap(), Distro::LinuxMint);
        assert_eq!("opensuse-leap".parse::<Distro>().unwrap(), Distro::OpenSUSE);
        assert_eq!(
            "opensuse-tumbleweed".parse::<Distro>().unwrap(),
            Distro::OpenSUSE
        );
    }

    #[test]
    fn test_distro_from_str_invalid() {
        assert!("invalid".parse::<Distro>().is_err());
        assert!("".parse::<Distro>().is_err());
        assert!("ubuntuo".parse::<Distro>().is_err());
    }

    #[test]
    fn test_distro_is_none() {
        #[cfg(target_os = "linux")]
        {
            let distro = detect_distro();
            assert!(!distro.is_none() || distro == Distro::Unknown);
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert!(detect_distro().is_none());
        }
    }

    #[test]
    fn test_distro_is_unknown() {
        let distro = detect_distro();
        #[cfg(target_os = "linux")]
        {
            assert!(!distro.is_unknown() || distro == Distro::Unknown);
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert!(distro.is_none());
            assert!(!distro.is_unknown());
        }
    }

    #[test]
    fn test_os_display_impl() {
        let os = OS::Windows;
        assert!(format!("{:?}", os).contains("Windows"));
    }

    #[test]
    fn test_distro_display_impl() {
        let distro = Distro::Ubuntu;
        assert!(format!("{:?}", distro).contains("Ubuntu"));
    }
}
