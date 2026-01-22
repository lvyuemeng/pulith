# pulith-platform

Cross-platform system utilities. OS, architecture, shell, and path helpers.

## API

```rust
// OS detection
os::detect() -> OS;                    // Windows, Macos, Linux(Distro)
os::distro() -> Distro;                // Debian, Ubuntu, etc.

// Architecture detection
arch::detect() -> Arch;                // X86, X86_64, ARM, ARM64
arch::target_triple(Arch) -> &'static str;

// Shell detection
shell::detect() -> Option<Shell>;      // Bash, Zsh, Fish, Powershell, etc.
shell::executable(Shell) -> Option<&'static str>;
shell::config_dir(Shell) -> Option<PathBuf>;

// Directory helpers
dir::user_home() -> Option<PathBuf>;
dir::user_config() -> Option<PathBuf>; // XDG_CONFIG_HOME or APPDATA
dir::user_data() -> Option<PathBuf>;
dir::user_temp() -> PathBuf;

// Environment
env::path_env() -> Option<Vec<PathBuf>>;
env::prepend_path(Path) -> io::Result<()>;
```

## Example

```rust
use pulith_platform::{os, arch, shell, dir};

match os::detect() {
    OS::Windows => println!("Windows"),
    OS::Macos => println!("macOS"),
    OS::Linux(distro) => println!("Linux: {:?}", distro),
    _ => println!("Unknown"),
}
```

## Dependencies

```
query-shell, home
```

## Platform Notes

| OS | Shell | Config | Notes |
|----|-------|--------|-------|
| Windows | cmd/pwsh | APPDATA | Semicolon PATH |
| macOS | zsh/bash | ~/Library/Application Support | |
| Linux | varies | $XDG_CONFIG_HOME | /etc/os-release |
