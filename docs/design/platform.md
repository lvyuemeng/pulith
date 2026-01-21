# pulith-platform Design

## Overview

Cross-platform system utilities for resource management tools. Provides OS, architecture, shell, and path helpers.

## Scope

**Included:**
- OS and distribution detection (Windows, macOS, Linux distros)
- Architecture detection (x86, x64, ARM variants)
- Shell detection and config directory resolution
- PATH manipulation
- Home and temp directory resolution

**Excluded:**
- Async runtime (handled separately)
- Version parsing (separate crate)
- UI components (separate crate)
- Shell invocation (removed, use `std::process::Command` directly)

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
    pub fn detect() -> Option<Shell>;

    /// Get shell executable path
    pub fn executable(shell: Shell) -> Option<&'static str>;

    /// Get shell config directory
    pub fn config_dir(shell: Shell) -> Option<std::path::PathBuf>;
}

pub mod path {
    /// User's home directory
    pub fn user_home() -> Option<std::path::PathBuf>;

    /// User's configuration directory
    pub fn user_config() -> Option<std::path::PathBuf>;

    /// User's data directory
    pub fn user_data() -> Option<std::path::PathBuf>;

    /// System temporary directory
    pub fn user_temp() -> std::path::PathBuf;

    /// Get PATH environment variable as list of paths
    pub fn path_env() -> Option<Vec<std::path::PathBuf>>;

    /// Prepend path to PATH environment variable (process-level)
    ///
    /// # Safety
    /// Only safe in single-threaded programs.
    pub fn prepend_path(path: &std::path::Path) -> std::io::Result<()>;

    /// Remove path from PATH environment variable (process-level)
    ///
    /// # Safety
    /// Only safe in single-threaded programs.
    pub fn remove_path(path: &std::path::Path) -> std::io::Result<()>;
}
```

## Module Structure

```
pulith-platform/src/
├── lib.rs              # Public exports
├── os.rs               # OS and distribution detection
├── arch.rs             # Architecture detection
├── shell.rs            # Shell detection and config directories
└── path.rs             # Path and directory helpers
```

## Dependencies

```toml
[dependencies]
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

### Detect Shell Config Directory

```rust
use pulith_platform::shell::{self, Shell};

let shell = shell::detect();
if let Some(s) = shell {
    if let Some(config) = shell::config_dir(s) {
        println!("Shell config: {}", config.display());
    }
}
```

### Manipulate PATH

```rust
use pulith_platform::path;

let new_path = std::path::PathBuf::from("/custom/bin");
path::prepend_path(&new_path).unwrap();

let entries = path::path_env().unwrap();
assert!(entries[0] == new_path);
```

## Design Decisions

### Why query-shell?

- Accurate shell detection
- Supports many shell types
- Actively maintained

### Why No Shell Invocation?

- `std::process::Command` is sufficient for most cases
- Simpler API surface area
- Platform-specific shells handle quoting/escaping differently

### PATH Manipulation Safety

- Marked `unsafe` due to environment variable thread-safety concerns
- Only intended for single-threaded CLI tools
- Consider using `std::env::set_var` with proper synchronization in multithreaded contexts

## Platform-Specific Notes

### Windows
- PATH uses semicolon separator
- Shell is typically `cmd.exe` or `powershell.exe`
- Home is `USERPROFILE`
- Config uses `APPDATA`

### macOS
- Shell is typically `zsh` or `bash`
- Home is standard
- Config uses `~/Library/Application Support`

### Linux
- Distribution detection via `/etc/os-release`
- Shell varies by user preference
- XDG directories for config/data (`$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`)
