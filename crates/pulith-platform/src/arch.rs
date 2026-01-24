use crate::error::{Error, Result};
use crate::os::OS;
use std::str::FromStr;
use std::{env, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X86_64,
    ARM,
    ARM64,
    Unknown,
}

impl Arch {
    pub fn current() -> Self {
        match env::consts::ARCH {
            "x86" => Self::X86,
            "x86_64" => Self::X86_64,
            "arm" => Self::ARM,
            "aarch64" | "arm64" => Self::ARM64,
            _ => Self::Unknown,
        }
    }

    pub fn is_x86(&self) -> bool {
        matches!(self, Self::X86)
    }
    pub fn is_x86_64(&self) -> bool {
        matches!(self, Self::X86_64)
    }
    pub fn is_arm(&self) -> bool {
        matches!(self, Self::ARM)
    }
    pub fn is_arm64(&self) -> bool {
        matches!(self, Self::ARM64)
    }
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

impl FromStr for Arch {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "i386" | "i686" | "x86" => Ok(Self::X86),
            "x86_64" | "amd64" => Ok(Self::X86_64),
            "arm" | "armv7" | "armv7l" => Ok(Self::ARM),
            "aarch64" | "arm64" | "aarch64-unknown" => Ok(Self::ARM64),
            _ => Err(Error::UnknownArch(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTriple {
    pub arch: Arch,
    pub vendor: String,
    pub os: OS,
    pub env: Option<String>,
}

impl TargetTriple {
    pub fn host() -> Self {
        Self {
            arch: Arch::current(),
            vendor: "unknown".to_string(),
            os: OS::current(),
            env: env::var("CARGO_CFG_TARGET_ENV").ok(),
        }
    }

    fn arch_to_str(&self) -> &str {
        match self.arch {
            Arch::X86 => "i686",
            Arch::X86_64 => "x86_64",
            Arch::ARM => "armv7l",
            Arch::ARM64 => "aarch64",
            Arch::Unknown => "unknown",
        }
    }

    fn os_to_str(&self) -> &str {
        match self.os {
            OS::Windows => "windows",
            OS::MacOS => "darwin",
            OS::Linux => "linux",
            OS::FreeBSD => "freebsd",
            OS::Unknown => "unknown",
        }
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut triple = format!(
            "{}-{}-{}",
            self.arch_to_str(),
            self.vendor,
            self.os_to_str()
        );
        if let Some(env) = &self.env {
            triple.push('-');
            triple.push_str(env);
        }
        f.write_str(&triple)
    }
}

impl FromStr for TargetTriple {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        match parts[..] {
            [] | [_] => Err(Error::UnknownTriple(s.to_string())),
            [arch, os] => Ok(Self {
                arch: arch.parse()?,
                vendor: "unknown".to_string(),
                os: os.parse()?,
                env: None,
            }),
            [arch, vendor_os, os_env] => {
                if let Ok(os) = vendor_os.parse::<OS>() {
                    return Ok(Self {
                        arch: arch.parse()?,
                        vendor: "unknown".to_string(),
                        os,
                        env: Some(os_env.to_string()),
                    });
                } else if let Ok(os) = os_env.parse::<OS>() {
                    return Ok(Self {
                        arch: arch.parse()?,
                        vendor: vendor_os.to_string(),
                        os,
                        env: None,
                    });
                } else {
                    return Err(Error::UnknownTriple(s.to_string()));
                }
            }
            [arch, vendor, os, env] => Ok(Self {
                arch: arch.parse()?,
                vendor: vendor.to_string(),
                os: os.parse()?,
                env: Some(env.to_string()),
            }),
            _ => Err(Error::UnknownTriple(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_current_matches_cfg() {
        let arch = Arch::current();
        match arch {
            Arch::X86 => assert!(cfg!(any(target_arch = "x86",))),
            Arch::X86_64 => assert!(cfg!(target_arch = "x86_64")),
            Arch::ARM => assert!(cfg!(target_arch = "arm")),
            Arch::ARM64 => assert!(cfg!(any(target_arch = "aarch64", target_arch = "arm64ec"))),
            Arch::Unknown => assert!(!cfg!(any(
                target_arch = "x86",
                target_arch = "x86_64",
                target_arch = "arm",
                target_arch = "aarch64",
                target_arch = "arm64ec"
            ))),
        }
    }

    #[test]
    fn test_arch_is_x86() {
        let arch = Arch::current();
        assert_eq!(arch.is_x86(), cfg!(any(target_arch = "x86",)));
    }

    #[test]
    fn test_arch_is_x86_64() {
        let arch = Arch::current();
        assert_eq!(arch.is_x86_64(), cfg!(target_arch = "x86_64"));
    }

    #[test]
    fn test_arch_is_arm() {
        let arch = Arch::current();
        assert_eq!(arch.is_arm(), cfg!(target_arch = "arm"));
    }

    #[test]
    fn test_arch_is_arm64() {
        let arch = Arch::current();
        assert_eq!(
            arch.is_arm64(),
            cfg!(any(target_arch = "aarch64", target_arch = "arm64ec"))
        );
    }

    #[test]
    fn test_arch_from_str_x86_variants() {
        assert_eq!("x86".parse::<Arch>().unwrap(), Arch::X86);
        assert_eq!("i386".parse::<Arch>().unwrap(), Arch::X86);
        assert_eq!("i686".parse::<Arch>().unwrap(), Arch::X86);
    }

    #[test]
    fn test_arch_from_str_x86_64_variants() {
        assert_eq!("x86_64".parse::<Arch>().unwrap(), Arch::X86_64);
        assert_eq!("amd64".parse::<Arch>().unwrap(), Arch::X86_64);
    }

    #[test]
    fn test_arch_from_str_arm_variants() {
        assert_eq!("arm".parse::<Arch>().unwrap(), Arch::ARM);
        assert_eq!("armv7".parse::<Arch>().unwrap(), Arch::ARM);
        assert_eq!("armv7l".parse::<Arch>().unwrap(), Arch::ARM);
    }

    #[test]
    fn test_arch_from_str_arm64_variants() {
        assert_eq!("aarch64".parse::<Arch>().unwrap(), Arch::ARM64);
        assert_eq!("arm64".parse::<Arch>().unwrap(), Arch::ARM64);
    }

    #[test]
    fn test_arch_from_str_case_insensitive() {
        assert_eq!("X86_64".parse::<Arch>().unwrap(), Arch::X86_64);
        assert_eq!("ARM64".parse::<Arch>().unwrap(), Arch::ARM64);
        assert_eq!("AARCH64".parse::<Arch>().unwrap(), Arch::ARM64);
    }

    #[test]
    fn test_arch_from_str_invalid() {
        assert!("invalid".parse::<Arch>().is_err());
        assert!("".parse::<Arch>().is_err());
        assert!("x86_64_".parse::<Arch>().is_err());
    }

    #[test]
    fn test_target_triple_host() {
        let triple = TargetTriple::host();
        assert_eq!(triple.arch, Arch::current());
        assert_eq!(triple.os, OS::current());
        assert!(triple.vendor.is_empty() || triple.vendor == "unknown");
    }

    #[test]
    fn test_target_triple_parse_3_part() {
        let triple: TargetTriple = "x86_64-linux".parse().unwrap();
        assert_eq!(triple.arch, Arch::X86_64);
        assert_eq!(triple.vendor, "unknown");
        assert_eq!(triple.os, OS::Linux);
        assert_eq!(triple.env, None);
    }

    #[test]
    fn test_target_triple_parse_4_part() {
        let triple: TargetTriple = "x86_64-unknown-linux-gnu".parse().unwrap();
        assert_eq!(triple.arch, Arch::X86_64);
        assert_eq!(triple.vendor, "unknown");
        assert_eq!(triple.os, OS::Linux);
        assert_eq!(triple.env, Some("gnu".to_string()));
    }

    #[test]
    fn test_target_triple_parse_musl() {
        let triple: TargetTriple = "aarch64-unknown-linux-musl".parse().unwrap();
        assert_eq!(triple.arch, Arch::ARM64);
        assert_eq!(triple.vendor, "unknown");
        assert_eq!(triple.os, OS::Linux);
        assert_eq!(triple.env, Some("musl".to_string()));
    }

    #[test]
    fn test_target_triple_parse_darwin() {
        let triple: TargetTriple = "x86_64-apple-darwin".parse().unwrap();
        assert_eq!(triple.arch, Arch::X86_64);
        assert_eq!(triple.vendor, "apple");
        assert_eq!(triple.os, OS::MacOS);
        assert_eq!(triple.env, None);
    }

    #[test]
    fn test_target_triple_parse_windows() {
        let triple: TargetTriple = "x86_64-pc-windows-msvc".parse().unwrap();
        assert_eq!(triple.arch, Arch::X86_64);
        assert_eq!(triple.vendor, "pc");
        assert_eq!(triple.os, OS::Windows);
        assert_eq!(triple.env, Some("msvc".to_string()));
    }

    #[test]
    fn test_target_triple_parse_invalid() {
        assert!("invalid".parse::<TargetTriple>().is_err());
        assert!("x86_64".parse::<TargetTriple>().is_err());
        assert!("x86_64-linux-gnu-extra".parse::<TargetTriple>().is_err());
    }

    #[test]
    fn test_target_triple_to_string() {
        let triple = TargetTriple {
            arch: Arch::X86_64,
            vendor: "unknown".to_string(),
            os: OS::Linux,
            env: Some("gnu".to_string()),
        };
        assert_eq!(triple.to_string(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_target_triple_to_string_no_env() {
        let triple = TargetTriple {
            arch: Arch::X86_64,
            vendor: "unknown".to_string(),
            os: OS::Linux,
            env: None,
        };
        assert_eq!(triple.to_string(), "x86_64-unknown-linux");
    }

    #[test]
    fn test_target_triple_partial_eq() {
        let triple1: TargetTriple = "x86_64-linux-gnu".parse().unwrap();
        let triple2: TargetTriple = "x86_64-linux-gnu".parse().unwrap();
        assert_eq!(triple1, triple2);
    }

    #[test]
    fn test_arch_display_impl() {
        let arch = Arch::X86_64;
        assert!(format!("{:?}", arch).contains("X86_64"));
    }
}
