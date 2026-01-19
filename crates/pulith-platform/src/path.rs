//! Platform-specific path and directory helpers.

use std::env;
use std::path::{Path, PathBuf};

/// User's home directory.
pub fn user_home() -> Option<PathBuf> {
    home::home_dir()
}

/// User's configuration directory.
///
/// - Windows: `APPDATA`
/// - macOS: `~/Library/Application Support`
/// - Linux: `$XDG_CONFIG_HOME` or `~/.config`
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

/// User's data directory.
///
/// - Windows: `LOCALAPPDATA`
/// - macOS: `~/Library/Application Support`
/// - Linux: `$XDG_DATA_HOME` or `~/.local/share`
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

/// System temporary directory.
pub fn user_temp() -> PathBuf {
    env::temp_dir()
}

/// Get PATH environment variable as a list of paths.
pub fn path_env() -> Option<Vec<PathBuf>> {
    let path_var = if cfg!(target_os = "windows") {
        "Path"
    } else {
        "PATH"
    };
    env::var_os(path_var).map(|v| env::split_paths(&v).collect())
}

/// Prepend a path to the PATH environment variable (process-level).
///
/// # Safety
///
/// `env::set_var` is only safe to call in single-threaded programs.
/// On multi-threaded programs, other threads may read/write the environment
/// simultaneously through libc functions, causing undefined behavior.
pub fn prepend_path(path: &Path) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = path_env().unwrap_or_default().into_iter().filter(|p| p!= path).collect();

    entries.insert(0, path.to_path_buf());

    let new_path = env::join_paths(entries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    // SAFETY: Only safe in single-threaded programs.
    // See module-level safety documentation.
    unsafe { env::set_var("PATH", new_path) };
    Ok(())
}

/// Remove a path from the PATH environment variable (process-level).
///
/// # Safety
///
/// `env::set_var` is only safe to call in single-threaded programs.
/// On multi-threaded programs, other threads may read/write the environment
/// simultaneously through libc functions, causing undefined behavior.
pub fn remove_path(path: &Path) -> std::io::Result<()> {
    let entries: Vec<PathBuf> = path_env()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p != path)
        .collect();

    let new_path = env::join_paths(entries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    // SAFETY: Only safe in single-threaded programs.
    // See module-level safety documentation.
    unsafe { env::set_var("PATH", new_path) };
    Ok(())
}
