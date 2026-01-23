use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};
use crate::options::ExtractOptions;

/// Result of sanitizing an archive entry path.
#[derive(Clone, Debug)]
pub struct SanitizedPath {
    pub original: PathBuf,
    pub resolved: PathBuf,
}

/// Sanitize a path for extraction using the provided options.
///
/// Combines path normalization, component stripping, and security validation.
pub fn sanitize_path_with_options<P: AsRef<Path>, B: AsRef<Path>>(
    entry_path: P,
    base: B,
    options: &ExtractOptions,
) -> Result<SanitizedPath> {
    let entry_path = entry_path.as_ref();
    let base = base.as_ref();
    let normalized = normalize_path(entry_path)?;

    // Reject absolute paths (zip-slip protection)
    if normalized.is_absolute() {
        return Err(Error::ZipSlip {
            entry: entry_path.to_path_buf(),
            resolved: normalized,
        });
    }

    // Strip components if requested
    let processed = if options.strip_components > 0 {
        strip_components(&normalized, options.strip_components)?
    } else {
        normalized
    };

    // Resolve against base and normalize
    let resolved = normalize_path(&base.join(processed))?;

    // Ensure result doesn't escape base directory
    if !resolved.starts_with(base) {
        return Err(Error::ZipSlip {
            entry: entry_path.to_path_buf(),
            resolved,
        });
    }

    Ok(SanitizedPath {
        original: entry_path.to_path_buf(),
        resolved,
    })
}

/// Sanitize a symlink target path using the provided options.
pub fn sanitize_symlink_target_with_options<P: AsRef<Path>, L: AsRef<Path>, B: AsRef<Path>>(
    target: P,
    symlink_location: L,
    base: B,
    options: &ExtractOptions,
) -> Result<PathBuf> {
    let target = target.as_ref();
    let symlink_location = symlink_location.as_ref();
    let base = base.as_ref();

    // Reject absolute symlink targets
    if target.is_absolute() {
        return Err(Error::AbsoluteSymlinkTarget {
            target: target.to_path_buf(),
            symlink: symlink_location.to_path_buf(),
        });
    }

    let normalized = normalize_path(target)?;

    // Strip components if requested
    let processed = if options.strip_components > 0 {
        strip_components(&normalized, options.strip_components)?
    } else {
        normalized
    };

    // Resolve relative to symlink location
    let resolved = symlink_location
        .parent()
        .map(|p| p.join(&processed))
        .unwrap_or(processed);

    // Make absolute and normalize
    let absolute = if resolved.is_absolute() {
        resolved
    } else {
        base.join(resolved)
    };
    let final_path = normalize_path(&absolute)?;

    // Ensure symlink doesn't escape base directory
    if !final_path.starts_with(base) {
        return Err(Error::SymlinkEscape {
            target: target.to_path_buf(),
            resolved: final_path,
        });
    }

    Ok(final_path)
}

/// Strip leading path components.
fn strip_components(path: &Path, count: usize) -> Result<PathBuf> {
    let components: Vec<_> = path.components().collect();
    if components.len() <= count {
        return Err(Error::NoComponentsRemaining {
            original: path.to_path_buf(),
            count,
        });
    }
    Ok(components[count..].iter().collect())
}

/// Normalize path separators and resolve relative components.
fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            Component::ParentDir => { result.pop(); }
            Component::Normal(part) => result.push(part),
            Component::RootDir => result.push("/"),
            Component::Prefix(prefix) => result.push(prefix.as_os_str()),
            Component::CurDir => {} // ignore current dir
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
    fn basic_path_sanitization() {
        let options = ExtractOptions::default();
        let result = sanitize_path_with_options("bin/tool", test_base_path(), &options).unwrap();
        assert_eq!(result.original, Path::new("bin/tool"));
        assert!(result.resolved.starts_with(test_base_path()));
    }

    #[test]
    fn path_with_component_stripping() {
        let options = ExtractOptions::default().strip_components(1);
        let result = sanitize_path_with_options("tool-1.0/bin/tool", test_base_path(), &options).unwrap();

        // Check that the stripped path contains the expected components
        let resolved_str = result.resolved.to_string_lossy();
        assert!(resolved_str.contains("bin") && resolved_str.contains("tool"));
        assert!(!resolved_str.contains("tool-1.0"));

        // Verify the relative part after base path
        let relative_part = result.resolved.strip_prefix(test_base_path()).unwrap();
        assert_eq!(relative_part, Path::new("bin/tool"));
    }

    #[test]
    fn zip_slip_protection() {
        let options = ExtractOptions::default();
        let malicious_path = if cfg!(windows) { "C:\\etc\\passwd" } else { "/etc/passwd" };
        let result = sanitize_path_with_options(malicious_path, test_base_path(), &options);
        assert!(matches!(result, Err(Error::ZipSlip { .. })));
    }

    #[test]
    fn symlink_target_sanitization() {
        let options = ExtractOptions::default();
        let target = "../lib";
        let symlink_location = test_base_path().join("bin/mylink");
        let result = sanitize_symlink_target_with_options(
            target, symlink_location, test_base_path(), &options
        ).unwrap();
        assert!(result.starts_with(test_base_path()));
    }

    #[test]
    fn symlink_absolute_path_rejected() {
        let options = ExtractOptions::default();
        let absolute_target = if cfg!(windows) { "C:\\etc\\passwd" } else { "/etc/passwd" };
        let symlink_location = test_base_path().join("bin/mylink");
        let result = sanitize_symlink_target_with_options(
            absolute_target, symlink_location, test_base_path(), &options
        );
        assert!(matches!(result, Err(Error::AbsoluteSymlinkTarget { .. })));
    }

    #[test]
    fn path_normalization() {
        let result = normalize_path(Path::new("foo//bar\\baz/../qux")).unwrap();
        assert_eq!(result, Path::new("foo/bar/qux"));
    }

    #[test]
    fn component_stripping() {
        let result = strip_components(Path::new("a/b/c/d"), 2).unwrap();
        assert_eq!(result, Path::new("c/d"));
    }
}
