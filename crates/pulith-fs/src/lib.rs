mod error;
mod transaction;
mod workspace;

pub use error::{Error, Result, from_io};

use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
const DEFAULT_PERMISSIONS: u32 = 0o644;

#[cfg(not(unix))]
const DEFAULT_PERMISSIONS: u32 = 0;

#[derive(Clone, Copy, Debug)]
pub struct AtomicWriteOptions {
    permissions: u32,
    prefix:      &'static str,
    suffix:      &'static str,
}

impl Default for AtomicWriteOptions {
    fn default() -> Self { Self::new() }
}

impl AtomicWriteOptions {
    pub fn new() -> Self {
        Self {
            permissions: DEFAULT_PERMISSIONS,
            prefix:      ".",
            suffix:      ".tmp",
        }
    }

    #[cfg(unix)]
    pub fn permissions(mut self, permissions: u32) -> Self {
        self.permissions = permissions;
        self
    }

    #[cfg(not(unix))]
    pub fn permissions(self, _permissions: u32) -> Self { self }

    pub fn prefix(mut self, prefix: &'static str) -> Self {
        self.prefix = prefix;
        self
    }

    pub fn suffix(mut self, suffix: &'static str) -> Self {
        self.suffix = suffix;
        self
    }

    #[cfg(unix)]
    pub fn into_permissions(self) -> Option<std::fs::Permissions> {
        Some(std::fs::Permissions::from_mode(self.permissions))
    }

    #[cfg(not(unix))]
    pub fn into_permissions(self) -> Option<std::fs::Permissions> { None }

    pub fn prefix_str(&self) -> &'static str { self.prefix }

    pub fn suffix_str(&self) -> &'static str { self.suffix }
}

#[derive(Clone, Copy, Debug)]
pub struct ReplaceDirOptions {
    retry_count:    u32,
    retry_delay_ms: u64,
}

impl Default for ReplaceDirOptions {
    fn default() -> Self { Self::new() }
}

impl ReplaceDirOptions {
    pub fn new() -> Self {
        Self {
            retry_count:    64,
            retry_delay_ms: 8,
        }
    }

    pub fn retry_count(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }

    pub fn retry_delay_ms(mut self, retry_delay_ms: u64) -> Self {
        self.retry_delay_ms = retry_delay_ms;
        self
    }

    pub fn get_retry_count(&self) -> u32 { self.retry_count }

    pub fn get_retry_delay_ms(&self) -> u64 { self.retry_delay_ms }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallbackStrategy {
    Error,
    Copy,
}

impl Default for FallbackStrategy {
    fn default() -> Self { Self::Error }
}

#[derive(Clone, Copy, Debug)]
pub struct HardlinkOrCopyOptions {
    fallback:    FallbackStrategy,
    #[cfg(unix)]
    permissions: u32,
}

impl Default for HardlinkOrCopyOptions {
    fn default() -> Self { Self::new() }
}

impl HardlinkOrCopyOptions {
    #[cfg(unix)]
    pub fn new() -> Self {
        Self {
            fallback:    FallbackStrategy::default(),
            permissions: DEFAULT_PERMISSIONS,
        }
    }

    #[cfg(windows)]
    pub fn new() -> Self {
        Self {
            fallback: FallbackStrategy::default(),
        }
    }

    pub fn fallback(mut self, fallback: FallbackStrategy) -> Self {
        self.fallback = fallback;
        self
    }

    #[cfg(unix)]
    pub fn permissions(mut self, permissions: u32) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn get_fallback(&self) -> FallbackStrategy { self.fallback }

    #[cfg(unix)]
    pub fn into_permissions(self) -> Option<std::fs::Permissions> {
        Some(std::fs::Permissions::from_mode(self.permissions))
    }

    #[cfg(windows)]
    pub fn into_permissions(self) -> Option<std::fs::Permissions> { None }
}

pub fn atomic_write(
    path: impl AsRef<Path>,
    content: &[u8],
    options: AtomicWriteOptions,
) -> Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or(Path::new(""));
    let prefix = options.prefix_str();
    let suffix = options.suffix_str();

    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp_name = format!("{}{}{}", prefix, file_name, suffix);
    let tmp_path = parent.join(tmp_name);

    std::fs::write(&tmp_path, content).map_err(error::from_io)?;

    if let Some(perms) = options.into_permissions() {
        std::fs::set_permissions(&tmp_path, perms).map_err(error::from_io)?;
    }

    std::fs::rename(&tmp_path, path).map_err(error::from_io)?;

    if cfg!(not(windows)) {
        if let Some(parent_perms) = parent.metadata().map(|m| m.permissions()).ok() {
            let _ = std::fs::set_permissions(path, parent_perms);
        }
    }

    Ok(())
}

#[cfg(unix)]
pub fn atomic_symlink(target: impl AsRef<Path>, link: impl AsRef<Path>) -> Result<()> {
    use nix::unistd::symlinkat;

    let target = target.as_ref();
    let link = link.as_ref();

    let tmp_link = link.with_extension(".tmp");

    symlinkat(target, None, &tmp_link).map_err(|_| Error::Failed)?;
    std::fs::rename(&tmp_link, link).map_err(error::map_io_error)?;

    Ok(())
}

#[cfg(unix)]
pub fn replace_dir(
    src: impl AsRef<Path>,
    _dest: impl AsRef<Path>,
    _options: ReplaceDirOptions,
) -> Result<()> {
    std::fs::rename(src.as_ref(), _dest.as_ref()).map_err(error::map_io_error)
}

#[cfg(windows)]
pub fn atomic_symlink(target: impl AsRef<Path>, link: impl AsRef<Path>) -> Result<()> {
    use std::os::windows::prelude::OsStrExt;
    use windows::Win32::Storage::FileSystem::{CreateSymbolicLinkW, SYMBOLIC_LINK_FLAGS};
    use windows::core::PCWSTR;

    let target_wide: Vec<u16> = target
        .as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let link_wide: Vec<u16> = link
        .as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        CreateSymbolicLinkW(
            PCWSTR::from_raw(link_wide.as_ptr()),
            PCWSTR::from_raw(target_wide.as_ptr()),
            SYMBOLIC_LINK_FLAGS(1),
        )
    };

    if !result {
        return Err(Error::Failed);
    }

    Ok(())
}

#[cfg(windows)]
pub fn replace_dir(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    options: ReplaceDirOptions,
) -> Result<()> {
    use std::os::windows::prelude::OsStrExt;
    use std::thread;
    use std::time::Duration;
    use windows::Win32::Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW};
    use windows::core::PCWSTR;

    let src_wide: Vec<u16> = src
        .as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let dest_wide: Vec<u16> = dest
        .as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut delay = options.get_retry_delay_ms();

    for attempt in 0..options.get_retry_count() {
        let result = unsafe {
            MoveFileExW(
                PCWSTR::from_raw(src_wide.as_ptr()),
                PCWSTR::from_raw(dest_wide.as_ptr()),
                MOVE_FILE_FLAGS(2),
            )
        };

        if result.is_ok() {
            return Ok(());
        }

        if attempt < options.get_retry_count() - 1 {
            thread::sleep(Duration::from_millis(delay));
            delay *= 2;
            continue;
        }
        return Err(Error::RetryLimitExceeded);
    }

    Err(Error::RetryLimitExceeded)
}

pub fn hardlink_or_copy(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    options: HardlinkOrCopyOptions,
) -> Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();

    match std::fs::hard_link(src, dest) {
        Ok(()) => return Ok(()),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::CrossesDevices {
                return Err(error::from_io(e));
            }
        }
    }

    match options.get_fallback() {
        FallbackStrategy::Error => Err(Error::CrossDeviceHardlink),
        FallbackStrategy::Copy => {
            if src.is_dir() {
                copy_dir_all(src, dest)?;
            } else {
                std::fs::copy(src, dest).map_err(error::from_io)?;
            }

            if let Some(perms) = options.into_permissions() {
                std::fs::set_permissions(dest, perms).map_err(error::from_io)?;
            }

            Ok(())
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();

    std::fs::create_dir_all(dest).map_err(error::from_io)?;

    for entry in std::fs::read_dir(src).map_err(error::from_io)? {
        let entry = entry.map_err(error::from_io)?;
        let ty = entry.file_type().map_err(error::from_io)?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(error::from_io)?;
        }
    }

    Ok(())
}

pub fn atomic_read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    std::fs::read(path.as_ref()).map_err(error::from_io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(unix)]
    #[test]
    fn test_atomic_write() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("test.txt");
        atomic_write(&path, b"data", AtomicWriteOptions::new())?;
        assert_eq!(std::fs::read(&path)?, b"data");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_symlink() -> Result<()> {
        let dir = tempdir()?;
        let target = dir.path().join("target");
        let link = dir.path().join("link");

        std::fs::write(&target, "data")?;
        atomic_symlink(&target, &link)?;

        assert!(link.is_symlink());
        assert_eq!(std::fs::read_to_string(link)?, "data");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_replace_dir() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");

        std::fs::create_dir_all(&src)?;

        replace_dir(&src, &dest, ReplaceDirOptions::new())?;

        assert!(dest.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_hardlink_or_copy_cross_device() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");

        std::fs::write(&src, "data")?;

        let options = HardlinkOrCopyOptions::new().fallback(FallbackStrategy::Copy);
        hardlink_or_copy(&src, &dest, options)?;

        assert_eq!(std::fs::read(&dest)?, b"data");
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn test_junction_point() -> Result<()> {
        let dir = tempdir()?;
        let target = dir.path().join("target");
        let junction = dir.path().join("junction");

        std::fs::create_dir_all(&target)?;
        atomic_symlink(&target, &junction)?;

        assert!(junction.is_dir());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn test_replace_dir() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");

        std::fs::create_dir_all(&src)?;

        let options = ReplaceDirOptions::new().retry_count(10).retry_delay_ms(8);
        replace_dir(&src, &dest, options)?;

        assert!(dest.exists());
        Ok(())
    }
}
