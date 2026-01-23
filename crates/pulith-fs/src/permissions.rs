use crate::{Error, Result};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Cross-platform file permission modes.
///
/// This type provides a platform-agnostic way to specify file permissions
/// during atomic write operations.
pub enum PermissionMode {
    /// Use system default permissions.
    /// On Unix: Uses the process's umask
    /// On Windows: Uses Windows default permissions
    #[default]
    Inherit,

    /// Write-protected (read-only) mode.
    /// On Unix: Sets `0o444` (r--r--r--)
    /// On Windows: Sets the `readonly` file attribute
    ReadOnly,

    /// Custom Unix mode bits (e.g., `0o755` for `rwxr-xr-x`).
    /// On Windows: This is ignored (no-op) as Windows uses ACLs instead.
    ///
    /// # Unix Mode Examples
    /// - `0o755`: `rwxr-xr-x` (owner: all, group/others: read+execute)
    /// - `0o644`: `rw-r--r--` (owner: read+write, group/others: read)
    /// - `0o777`: `rwxrwxrwx` (everyone: all permissions)
    Custom(u32),
}

impl PermissionMode {
    /// Apply the permission mode to a file path.
    ///
    /// # Platform Behavior
    /// - **Unix**: Sets mode bits via `PermissionsExt::from_mode()`
    /// - **Windows**: Sets `readonly` attribute for `ReadOnly`, no-op for others
    ///
    /// # Errors
    /// Returns an error if the file does not exist or permissions cannot be set.
    pub fn apply_to_path(self, path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = match self {
                Self::Inherit => return Ok(()),
                Self::ReadOnly => 0o444,
                Self::Custom(m) => m,
            };
            let perms = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(path, perms).map_err(|e| Error::Write {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        #[cfg(windows)]
        {
            match self {
                Self::Inherit => {}
                Self::ReadOnly => {
                    let mut perms = std::fs::metadata(path)
                        .map_err(|e| Error::Write {
                            path: path.to_path_buf(),
                            source: e,
                        })?
                        .permissions();
                    perms.set_readonly(true);
                    std::fs::set_permissions(path, perms).map_err(|e| Error::Write {
                        path: path.to_path_buf(),
                        source: e,
                    })?;
                }
                Self::Custom(_) => {}
            }
        }

        Ok(())
    }

    /// Convert Unix mode bits to PermissionMode.
    ///
    /// Returns `None` if the mode bits are `None` (not available).
    #[cfg(unix)]
    pub fn from_unix_mode(mode: Option<u32>) -> Option<Self> {
        mode.map(Self::Custom)
    }

    /// Extract Unix mode bits if this is a `Custom` variant.
    ///
    /// Returns `None` for `Inherit` or `ReadOnly` variants.
    pub fn to_unix_mode(self) -> Option<u32> {
        match self {
            Self::Custom(m) => Some(m),
            Self::Inherit | Self::ReadOnly => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_mode_default() {
        assert_eq!(PermissionMode::default(), PermissionMode::Inherit);
    }

    #[test]
    fn permission_mode_to_unix_mode() {
        assert_eq!(PermissionMode::Custom(0o755).to_unix_mode(), Some(0o755));
        assert_eq!(PermissionMode::Custom(0o644).to_unix_mode(), Some(0o644));
        assert_eq!(PermissionMode::Inherit.to_unix_mode(), None);
        assert_eq!(PermissionMode::ReadOnly.to_unix_mode(), None);
    }

    #[cfg(unix)]
    #[test]
    fn permission_mode_from_unix_mode() {
        assert_eq!(
            PermissionMode::from_unix_mode(Some(0o755)),
            Some(PermissionMode::Custom(0o755))
        );
        assert_eq!(
            PermissionMode::from_unix_mode(Some(0o644)),
            Some(PermissionMode::Custom(0o644))
        );
        assert_eq!(PermissionMode::from_unix_mode(None), None);
    }
}
