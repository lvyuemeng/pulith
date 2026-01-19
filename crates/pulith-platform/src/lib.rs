//! Cross-platform system utilities for resource management.
//!
//! Provides OS, architecture, shell, and path helpers.

pub mod arch;
pub mod os;
pub mod path;
pub mod shell;

#[cfg(test)]
mod tests {
    use crate::arch::Arch;
    use crate::os::OS;
    use crate::shell::Shell;

    #[test]
    fn test_os_detection() {
        let os = crate::os::detect();
        match os {
            OS::Windows | OS::Macos | OS::Linux(_) | OS::Unknown => {}
        }
    }

    #[test]
    fn test_arch_detection() {
        let arch = crate::arch::detect();
        match arch {
            Arch::X86 | Arch::X86_64 | Arch::ARM | Arch::ARM64 | Arch::Unknown => {}
        }
    }

    #[test]
    fn test_shell_executable() {
        assert_eq!(crate::shell::executable(Shell::Bash), Some("bash"));
        assert_eq!(crate::shell::executable(Shell::Unknown), None);
    }

    #[test]
    fn test_user_home() {
        let home = crate::path::user_home();
        // May be None in some environments
        if let Some(p) = home {
            assert!(p.exists() || p.to_string_lossy().len() > 0);
        }
    }

    #[test]
    fn test_temp_dir() {
        let temp = crate::path::user_temp();
        assert!(temp.exists() || temp.to_string_lossy().len() > 0);
    }
}
