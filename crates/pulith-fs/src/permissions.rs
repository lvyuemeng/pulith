use crate::{Error, Result};
use std::path::Path;

/// Cross-platform file permission modes with consistent behavior across platforms.
///
/// This type provides a platform-agnostic way to specify file permissions
/// during atomic write operations, with sensible defaults that work well
/// on both Unix and Windows systems.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PermissionMode {
    /// Use system default permissions.
    ///
    /// On Unix: Uses the process's umask
    /// On Windows: Uses Windows default permissions
    #[default]
    Inherit,

    /// Read-only mode for files.
    ///
    /// On Unix: Sets `0o444` (r--r--r--)
    /// On Windows: Sets the `readonly` file attribute
    ReadOnly,

    /// Executable file permissions.
    ///
    /// On Unix: Sets `0o755` (rwxr-xr-x) - owner can read/write/execute, others can read/execute
    /// On Windows: Sets `readonly = false` (allows execution)
    Executable,

    /// Read-write file permissions.
    ///
    /// On Unix: Sets `0o644` (rw-r--r--) - owner can read/write, others can read
    /// On Windows: Sets `readonly = false` (allows modification)
    ReadWrite,

    /// Directory permissions.
    ///
    /// On Unix: Sets `0o755` (rwxr-xr-x) - owner can read/write/execute, others can read/execute
    /// On Windows: Sets `readonly = false` (allows directory operations)
    Directory,

    /// Custom permissions with platform-specific handling.
    ///
    /// On Unix: Uses the provided mode bits (e.g., `0o755` for `rwxr-xr-x`)
    /// On Windows: Maps Unix permissions to Windows equivalents:
    ///   - `0o7xx` (executable): `readonly = false`
    ///   - `0o6xx` (read-write): `readonly = false`
    ///   - `0o4xx` (read-only): `readonly = true`
    ///   - Other modes: `readonly = false`
    Custom(CustomPermissions),
}

/// Custom permission specification that can be mapped across platforms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CustomPermissions {
    /// Unix permission mode bits (e.g., 0o755, 0o644)
    pub unix_mode: u32,
}

impl CustomPermissions {
    /// Create custom permissions from Unix mode bits.
    ///
    /// # Examples
    /// ```
    /// use pulith_fs::permissions::CustomPermissions;
    ///
    /// let perms = CustomPermissions::from_unix_mode(0o755); // rwxr-xr-x
    /// let perms = CustomPermissions::from_unix_mode(0o644); // rw-r--r--
    /// ```
    pub fn from_unix_mode(mode: u32) -> Self {
        Self { unix_mode: mode }
    }

    /// Get the Unix mode bits.
    pub fn to_unix_mode(self) -> u32 {
        self.unix_mode
    }

    /// Check if these permissions allow execution.
    pub fn is_executable(self) -> bool {
        (self.unix_mode & 0o111) != 0
    }

    /// Check if these permissions allow writing.
    pub fn is_writable(self) -> bool {
        (self.unix_mode & 0o222) != 0
    }

    /// Check if these permissions are read-only.
    pub fn is_readonly(self) -> bool {
        !self.is_writable() && (self.unix_mode & 0o444) != 0
    }
}

impl PermissionMode {
    /// Apply the permission mode to a file or directory path.
    ///
    /// # Platform Behavior
    /// - **Unix**: Sets mode bits via `PermissionsExt::from_mode()`
    /// - **Windows**: Sets `readonly` attribute based on permission type
    ///
    /// # Errors
    /// Returns an error if the file/directory does not exist or permissions cannot be set.
    pub fn apply_to_path(self, path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = match self {
                Self::Inherit => return Ok(()),
                Self::ReadOnly => 0o444,
                Self::Executable => 0o755,
                Self::ReadWrite => 0o644,
                Self::Directory => 0o755,
                Self::Custom(custom) => custom.unix_mode,
            };
            let perms = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(path, perms).map_err(|e| Error::Write {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        #[cfg(windows)]
        {
            let readonly = match self {
                Self::Inherit => return Ok(()),
                Self::ReadOnly => true,
                Self::Executable => false,
                Self::ReadWrite => false,
                Self::Directory => false,
                Self::Custom(custom) => {
                    // Map Unix permissions to Windows readonly flag
                    !custom.is_writable()
                }
            };

            let mut perms = std::fs::metadata(path)
                .map_err(|e| Error::Write {
                    path: path.to_path_buf(),
                    source: e,
                })?
                .permissions();
            perms.set_readonly(readonly);
            std::fs::set_permissions(path, perms).map_err(|e| Error::Write {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        Ok(())
    }

    /// Get the Unix mode bits if this represents a Unix permission.
    ///
    /// Returns `None` for `Inherit` or when the permission type doesn't
    /// have a meaningful Unix mode representation.
    pub fn to_unix_mode(self) -> Option<u32> {
        match self {
            Self::Inherit => None,
            Self::ReadOnly => Some(0o444),
            Self::Executable => Some(0o755),
            Self::ReadWrite => Some(0o644),
            Self::Directory => Some(0o755),
            Self::Custom(custom) => Some(custom.unix_mode),
        }
    }

    /// Check if this permission mode allows execution.
    pub fn is_executable(self) -> bool {
        match self {
            Self::Inherit => false, // Unknown without umask
            Self::ReadOnly => false,
            Self::Executable => true,
            Self::ReadWrite => false,
            Self::Directory => true,
            Self::Custom(custom) => custom.is_executable(),
        }
    }

    /// Check if this permission mode allows writing.
    pub fn is_writable(self) -> bool {
        match self {
            Self::Inherit => false, // Unknown without umask
            Self::ReadOnly => false,
            Self::Executable => true,
            Self::ReadWrite => true,
            Self::Directory => true,
            Self::Custom(custom) => custom.is_writable(),
        }
    }

    /// Check if this permission mode is read-only.
    pub fn is_readonly(self) -> bool {
        match self {
            Self::Inherit => false, // Unknown without umask
            Self::ReadOnly => true,
            Self::Executable => false,
            Self::ReadWrite => false,
            Self::Directory => false,
            Self::Custom(custom) => custom.is_readonly(),
        }
    }

    /// Create custom permissions from Unix mode bits.
    ///
    /// This is a convenience method for creating custom permissions.
    ///
    /// # Examples
    /// ```
    /// use pulith_fs::permissions::PermissionMode;
    ///
    /// let mode = PermissionMode::custom(0o755); // rwxr-xr-x
    /// let mode = PermissionMode::custom(0o644); // rw-r--r--
    /// ```
    pub fn custom(unix_mode: u32) -> Self {
        Self::Custom(CustomPermissions::from_unix_mode(unix_mode))
    }
}

impl From<CustomPermissions> for PermissionMode {
    fn from(custom: CustomPermissions) -> Self {
        Self::Custom(custom)
    }
}

impl From<u32> for PermissionMode {
    fn from(mode: u32) -> Self {
        Self::custom(mode)
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
        assert_eq!(
            PermissionMode::Custom(CustomPermissions::from_unix_mode(0o755)).to_unix_mode(),
            Some(0o755)
        );
        assert_eq!(
            PermissionMode::Custom(CustomPermissions::from_unix_mode(0o644)).to_unix_mode(),
            Some(0o644)
        );
        assert_eq!(PermissionMode::Inherit.to_unix_mode(), None);
        assert_eq!(PermissionMode::ReadOnly.to_unix_mode(), Some(0o444));
        assert_eq!(PermissionMode::Executable.to_unix_mode(), Some(0o755));
        assert_eq!(PermissionMode::ReadWrite.to_unix_mode(), Some(0o644));
        assert_eq!(PermissionMode::Directory.to_unix_mode(), Some(0o755));
    }

    #[test]
    fn custom_permissions_from_unix_mode() {
        let custom = CustomPermissions::from_unix_mode(0o755);
        assert_eq!(custom.to_unix_mode(), 0o755);
        assert!(custom.is_executable());
        assert!(custom.is_writable());
        assert!(!custom.is_readonly());
    }

    #[test]
    fn custom_permissions_readonly_detection() {
        let readonly = CustomPermissions::from_unix_mode(0o444);
        assert!(!readonly.is_executable());
        assert!(!readonly.is_writable());
        assert!(readonly.is_readonly());
    }

    #[test]
    fn permission_mode_executable_detection() {
        assert!(!PermissionMode::Inherit.is_executable());
        assert!(!PermissionMode::ReadOnly.is_executable());
        assert!(PermissionMode::Executable.is_executable());
        assert!(!PermissionMode::ReadWrite.is_executable());
        assert!(PermissionMode::Directory.is_executable());
        assert!(PermissionMode::custom(0o755).is_executable());
        assert!(!PermissionMode::custom(0o644).is_executable());
    }

    #[test]
    fn permission_mode_writable_detection() {
        assert!(!PermissionMode::Inherit.is_writable());
        assert!(!PermissionMode::ReadOnly.is_writable());
        assert!(PermissionMode::Executable.is_writable());
        assert!(PermissionMode::ReadWrite.is_writable());
        assert!(PermissionMode::Directory.is_writable());
        assert!(PermissionMode::custom(0o755).is_writable());
        assert!(PermissionMode::custom(0o644).is_writable());
        assert!(!PermissionMode::custom(0o444).is_writable());
    }

    #[test]
    fn permission_mode_readonly_detection() {
        assert!(!PermissionMode::Inherit.is_readonly());
        assert!(PermissionMode::ReadOnly.is_readonly());
        assert!(!PermissionMode::Executable.is_readonly());
        assert!(!PermissionMode::ReadWrite.is_readonly());
        assert!(!PermissionMode::Directory.is_readonly());
        assert!(!PermissionMode::custom(0o755).is_readonly());
        assert!(!PermissionMode::custom(0o644).is_readonly());
        assert!(PermissionMode::custom(0o444).is_readonly());
    }

    #[test]
    fn permission_mode_from_custom_permissions() {
        let custom = CustomPermissions::from_unix_mode(0o755);
        let mode: PermissionMode = custom.into();
        assert_eq!(mode, PermissionMode::Custom(custom));
    }

    #[test]
    fn permission_mode_from_u32() {
        let mode: PermissionMode = 0o755.into();
        assert_eq!(
            mode,
            PermissionMode::Custom(CustomPermissions::from_unix_mode(0o755))
        );
    }

    #[test]
    fn permission_mode_convenience_custom() {
        let mode = PermissionMode::custom(0o755);
        assert_eq!(
            mode,
            PermissionMode::Custom(CustomPermissions::from_unix_mode(0o755))
        );
    }
}
