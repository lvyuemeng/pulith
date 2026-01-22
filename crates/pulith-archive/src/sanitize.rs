use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct SanitizedEntry {
    pub original: PathBuf,
    pub resolved: PathBuf,
    pub symlink_target: Option<PathBuf>,
}

pub fn sanitize_path(entry_path: &Path, base: &Path) -> Result<SanitizedEntry> {
    let normalized = normalize_path(entry_path)?;

    if normalized.is_absolute() {
        return Err(Error::ZipSlip {
            entry: entry_path.to_path_buf(),
            resolved: normalized,
        });
    }

    let resolved = base.join(&normalized);
    let normalized_resolved = normalize_path(&resolved)?;

    if !normalized_resolved.starts_with(base) {
        return Err(Error::ZipSlip {
            entry: entry_path.to_path_buf(),
            resolved: normalized_resolved,
        });
    }

    Ok(SanitizedEntry {
        original: entry_path.to_path_buf(),
        resolved: normalized_resolved,
        symlink_target: None,
    })
}

pub fn sanitize_symlink_target(
    target: &Path,
    symlink_location: &Path,
    base: &Path,
) -> Result<PathBuf> {
    if target.is_absolute() {
        return Err(Error::AbsoluteSymlinkTarget {
            target: target.to_path_buf(),
            symlink: symlink_location.to_path_buf(),
        });
    }

    let normalized = normalize_path(target)?;

    let resolved = symlink_location
        .parent()
        .map(|p| p.join(&normalized))
        .unwrap_or_else(|| normalized.clone());

    let absolute_resolved = if resolved.is_absolute() {
        resolved
    } else {
        base.join(&resolved)
    };

    let normalized_resolved = normalize_path(&absolute_resolved)?;

    if !normalized_resolved.starts_with(base) {
        return Err(Error::SymlinkEscape {
            target: target.to_path_buf(),
            resolved: normalized_resolved,
        });
    }

    Ok(normalize_path(&normalized_resolved)?)
}

pub fn strip_path_components(path: &Path, count: usize) -> Result<PathBuf> {
    let normalized = normalize_path(path)?;
    let components: Vec<_> = normalized.components().collect();

    if components.len() <= count {
        return Err(Error::NoComponentsRemaining {
            original: path.to_path_buf(),
            count,
        });
    }

    let stripped: PathBuf = components[count..].iter().collect();
    Ok(stripped)
}

fn normalize_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.as_os_str().to_string_lossy();

    #[cfg(not(windows))]
    {
        if path.is_absolute() {
            let normalized = path_str.replace('\\', "/");
            return Ok(PathBuf::from(normalized));
        }
    }

    #[cfg(windows)]
    {
        let has_drive = path_str.chars().nth(1) == Some(':')
            || path_str.starts_with("//")
            || path_str.starts_with("\\\\")
            || (path_str.starts_with("/") && !path_str.starts_with("//"));
        if has_drive {
            let normalized = path_str.replace('\\', "/");
            return Ok(PathBuf::from(normalized));
        }
    }

    let normalized = path_str.replace('\\', "/");
    let normalized = PathBuf::from(normalized);

    let mut result = PathBuf::new();

    for component in normalized.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(part) => {
                result.push(part);
            }
            _ => {}
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_base_path() -> &'static Path {
        if cfg!(windows) {
            Path::new("C:/opt/myapp")
        } else {
            Path::new("/opt/myapp")
        }
    }

    #[test]
    fn sanitize_normal_path() {
        let entry = Path::new("bin/tool");
        let base = test_base_path();
        let result = sanitize_path(entry, base).unwrap();
        assert_eq!(result.original, Path::new("bin/tool"));
        assert!(result.resolved.starts_with(base));
        assert!(result.resolved.to_string_lossy().contains("bin/tool"));
    }

    #[test]
    fn sanitize_path_with_dot() {
        let entry = Path::new("bin/./tool");
        let base = test_base_path();
        let result = sanitize_path(entry, base).unwrap();
        assert!(result.resolved.starts_with(base));
        assert!(result.resolved.to_string_lossy().contains("bin/tool"));
    }

    #[test]
    fn sanitize_path_with_parent() {
        let entry = Path::new("bin/../lib/tool");
        let base = test_base_path();
        let result = sanitize_path(entry, base).unwrap();
        assert!(result.resolved.starts_with(base));
        assert!(result.resolved.to_string_lossy().contains("lib/tool"));
    }

    #[test]
    fn sanitize_zipslip_attack() {
        let entry = if cfg!(windows) {
            Path::new("C:\\etc\\passwd")
        } else {
            Path::new("/etc/passwd")
        };
        let base = test_base_path();
        let result = sanitize_path(entry, base);
        assert!(matches!(result, Err(Error::ZipSlip { .. })));
    }

    #[test]
    fn sanitize_deep_zipslip_attack() {
        let entry = if cfg!(windows) {
            Path::new("D:\\etc\\passwd")
        } else {
            Path::new("/home/user/etc/passwd")
        };
        let base = test_base_path();
        let result = sanitize_path(entry, base);
        assert!(matches!(result, Err(Error::ZipSlip { .. })));
    }

    #[test]
    fn sanitize_path_preserves_forward_slash() {
        let entry = Path::new("subdir/file.txt");
        let base = test_base_path();
        let result = sanitize_path(entry, base).unwrap();
        assert!(result.resolved.starts_with(base));
        assert!(result.resolved.to_string_lossy().contains('/'));
    }

    #[test]
    fn sanitize_symlink_relative_safe() {
        let target = Path::new("../lib");
        let symlink_location = test_base_path().join("bin/mylink");
        let base = test_base_path();
        let result = sanitize_symlink_target(target, &symlink_location, base).unwrap();
        assert!(result.starts_with(base));
        assert!(result.to_string_lossy().contains("lib"));
    }

    #[test]
    fn sanitize_symlink_absolute_rejected() {
        let target = if cfg!(windows) {
            PathBuf::from("C:\\etc\\passwd")
        } else {
            PathBuf::from("/etc/passwd")
        };
        let symlink_location = test_base_path().join("bin/mylink");
        let base = test_base_path();
        let result = sanitize_symlink_target(&target, &symlink_location, base);
        assert!(matches!(result, Err(Error::AbsoluteSymlinkTarget { .. })));
    }

    #[test]
    fn sanitize_symlink_escape_attack() {
        let target = if cfg!(windows) {
            Path::new("D:\\etc\\passwd")
        } else {
            Path::new("/etc/passwd")
        };
        let symlink_location = test_base_path().join("bin/mylink");
        let base = test_base_path();
        let result = sanitize_symlink_target(target, &symlink_location, base);
        assert!(matches!(
            result,
            Err(Error::SymlinkEscape { .. }) | Err(Error::AbsoluteSymlinkTarget { .. })
        ));
    }

    #[test]
    fn strip_path_components_single() {
        let path = Path::new("tool-1.0/bin/tool");
        let result = strip_path_components(path, 1).unwrap();
        assert_eq!(result, Path::new("bin/tool"));
    }

    #[test]
    fn strip_path_components_multiple() {
        let path = Path::new("tool-1.0/bin/subdir/tool");
        let result = strip_path_components(path, 2).unwrap();
        assert_eq!(result, Path::new("subdir/tool"));
    }

    #[test]
    fn strip_path_components_all_remaining() {
        let path = Path::new("tool-1.0/bin/tool");
        let result = strip_path_components(path, 2).unwrap();
        assert_eq!(result, Path::new("tool"));
    }

    #[test]
    fn strip_path_components_exceeds_length() {
        let path = Path::new("bin/tool");
        let result = strip_path_components(path, 5);
        assert!(matches!(result, Err(Error::NoComponentsRemaining { .. })));
    }

    #[test]
    fn strip_path_components_zero() {
        let path = Path::new("bin/tool");
        let result = strip_path_components(path, 0).unwrap();
        assert_eq!(result, Path::new("bin/tool"));
    }

    #[test]
    fn normalize_path_removes_double_slashes() {
        let path = Path::new("foo//bar//baz");
        let result = normalize_path(path).unwrap();
        assert_eq!(result, Path::new("foo/bar/baz"));
    }

    #[test]
    fn normalize_path_handles_mixed_separators() {
        let path = Path::new("foo\\bar/baz");
        let result = normalize_path(path).unwrap();
        assert_eq!(result, Path::new("foo/bar/baz"));
    }

    #[test]
    fn sanitize_path_with_nested_parent_dirs() {
        let entry = Path::new("a/b/../../c/./d");
        let base = test_base_path();
        let result = sanitize_path(entry, base).unwrap();
        assert!(result.resolved.starts_with(base));
        assert!(result.resolved.to_string_lossy().contains("c/d"));
    }
}
