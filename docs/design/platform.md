# pulith-platform Design

## Overview

Cross-platform system utilities for resource management tools. Provides OS, architecture, shell, and path helpers.

## Scope

**Included:**
- OS and distribution detection (Windows, macOS, Linux distros)
- Architecture detection (x86, x64, ARM variants)
- Shell detection and invocation
- PATH manipulation
- Home and temp directory resolution

**Excluded:**
- Async runtime (handled separately)
- Version parsing (separate crate)
- UI components (separate crate)

## Public API

```rust
pub mod os {
    /// Operating system types
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OS {
        Windows,
        Macos,
        Linux(Distro),
        Unknown,
    }

    /// Linux distribution types
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

    /// Detect current operating system
    pub fn detect() -> OS;

    /// Detect current Linux distribution (only valid if OS::Linux)
    pub fn distro() -> Distro;
}

pub mod arch {
    /// CPU architecture types
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Arch {
        X86,
        X86_64,
        ARM,
        ARM64,
        Unknown,
    }

    /// Detect current architecture
    pub fn detect() -> Arch;

    /// Convert to target triple format
    pub fn target_triple(arch: Arch) -> &'static str;
}

pub mod shell {
    /// Shell types
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

    /// Detect current shell
    pub fn detect() -> Shell;

    /// Get shell executable path
    pub fn executable(shell: Shell) -> Option<&'static str>;

    /// Invoke shell with command
    pub fn invoke(shell: Shell, command: &str) -> std::io::Result<std::process::ExitStatus>;
}

pub mod path {
    /// Platform-specific path helpers
    pub fn home_dir() -> Option<std::path::PathBuf>;

    pub fn temp_dir() -> std::path::PathBuf;

    pub fn config_dir() -> Option<std::path::PathBuf>;

    pub fn data_dir() -> Option<std::path::PathBuf>;

    pub fn cache_dir() -> Option<std::path::PathBuf>;

    /// Add path to PATH environment variable
    pub fn prepend_to_path(path: &std::path::Path) -> std::io::Result<()>;

    /// Remove path from PATH environment variable
    pub fn remove_from_path(path: &std::path::Path) -> std::io::Result<()>;

    /// Get current PATH as list of paths
    pub fn path_entries() -> Vec<std::path::PathBuf>;
}
```

## Module Structure

```
pulith-platform/src/
├── lib.rs              # Public exports
├── os.rs               # OS and distribution detection
├── arch.rs             # Architecture detection
├── shell.rs            # Shell detection and invocation
└── path.rs             # Path and directory helpers
```

## Dependencies

```toml
[dependencies]
sysinfo      # System information
query-shell  # Shell detection
home         # Home directory
```

> Exact versions in `crates/pulith-platform/Cargo.toml` for timeliness.

## Examples

### Detect OS

```rust
use pulith_platform::os::{self, OS};

let os = os::detect();
match os {
    OS::Windows => println!("Running on Windows"),
    OS::Macos => println!("Running on macOS"),
    OS::Linux(distro) => println!("Running on Linux: {:?}", distro),
    OS::Unknown => println!("Unknown OS"),
}
```

### Detect Shell and Execute

```rust
use pulith_platform::shell::{self, Shell};

let shell = shell::detect();
let status = shell::invoke(shell, "echo hello").unwrap();
assert!(status.success());
```

### Manipulate PATH

```rust
use pulith_platform::path;

let new_path = std::path::PathBuf::from("/custom/bin");
path::prepend_to_path(&new_path).unwrap();

let entries = path::path_entries();
assert!(entries[0] == new_path);
```

## Design Decisions

### Why sysinfo?

- Lightweight system information
- Cross-platform support
- No runtime dependencies

### Why query-shell?

- Accurate shell detection
- Supports many shell types
- Actively maintained

### Shell Invocation Design

- Simple command string (platform shell interprets)
- Returns exit status only (stdout/stderr handled by caller)
- For richer output, use `std::process::Command` directly

## Platform-Specific Notes

### Windows
- PATH uses semicolon separator
- Shell is typically `cmd.exe` or `powershell.exe`
- Home is `USERPROFILE`

### macOS
- Shell is typically `zsh` or `bash`
- Home is standard

### Linux
- Distribution detection via `/etc/os-release`
- Shell varies by user preference
- XDG directories for config/data/cache
