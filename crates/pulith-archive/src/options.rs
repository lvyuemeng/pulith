use pulith_fs::PermissionMode;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::Result;
use crate::error::Error;

/// Result of sanitizing an archive entry path.
#[derive(Clone, Debug)]
pub struct SanitizedPath {
    pub original: PathBuf,
    pub resolved: PathBuf,
}

#[derive(Clone, Default)]
pub struct ExtractOptions {
    pub perm_strategy: PermissionStrategy,
    pub hash_strategy: HashStrategy,
    pub strip_components: usize,
    pub expected_total_bytes: Option<u64>,
    pub on_progress: Option<Arc<dyn Fn(Progress) + Send + Sync>>,
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub bytes_processed: u64,
    pub total_bytes: Option<u64>,
    pub percentage: Option<f32>,
    pub current_file: Option<PathBuf>,
}

impl ExtractOptions {
    pub fn permission_strategy(mut self, strategy: PermissionStrategy) -> Self {
        self.perm_strategy = strategy;
        self
    }

    pub fn hash_strategy(mut self, strategy: HashStrategy) -> Self {
        self.hash_strategy = strategy;
        self
    }

    pub fn strip_components(mut self, n: usize) -> Self {
        self.strip_components = n;
        self
    }

    pub fn expected_total_bytes(mut self, bytes: u64) -> Self {
        self.expected_total_bytes = Some(bytes);
        self
    }

    pub fn on_progress(mut self, callback: Arc<dyn Fn(Progress) + Send + Sync>) -> Self {
        self.on_progress = Some(callback);
        self
    }

    /// Sanitize a path for extraction using the provided options.
    ///
    /// Combines path normalization, component stripping, and security validation.
    pub fn sanitize_path<P: AsRef<Path>, B: AsRef<Path>>(
        &self,
        entry_path: P,
        base: B,
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
        let processed = if self.strip_components > 0 {
            strip_components(&normalized, self.strip_components)?
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
    pub fn sanitize_symlink_target<P: AsRef<Path>, L: AsRef<Path>, B: AsRef<Path>>(
        &self,
        target: P,
        symlink_location: L,
        base: B,
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
        let processed = if self.strip_components > 0 {
            strip_components(&normalized, self.strip_components)?
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
}

impl Progress {
    pub fn percentage(&self) -> Option<f32> {
        self.total_bytes.map(|total| {
            if total == 0 {
                0.0
            } else {
                (self.bytes_processed as f32 / total as f32) * 100.0
            }
        })
    }
}

/// Hash computation strategies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HashStrategy {
    #[default]
    None,
    Sha256,
    Blake3,
}

impl HashStrategy {
    /// Compute hash from reader (streaming).
    pub fn compute<R: Read>(&self, mut reader: R) -> Result<Option<String>> {
        match self {
            Self::None => Ok(None),
            Self::Sha256 => {
                use sha2::Digest;
                let mut hasher = sha2::Sha256::new();
                let mut buffer = [0u8; 8192];

                loop {
                    let n = reader.read(&mut buffer).map_err(Error::from)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buffer[..n]);
                }

                Ok(Some(format!("{:x}", hasher.finalize())))
            }
            Self::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                let mut buffer = [0u8; 8192];

                loop {
                    let n = reader.read(&mut buffer).map_err(Error::from)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buffer[..n]);
                }

                Ok(Some(format!("{}", hasher.finalize())))
            }
        }
    }

    /// Compute hash from bytes (convenience).
    pub fn compute_from_bytes(&self, content: &[u8]) -> Option<String> {
        match self {
            Self::None => None,
            Self::Sha256 => {
                use sha2::Digest;
                let mut hasher = sha2::Sha256::new();
                hasher.update(content);
                Some(format!("{:x}", hasher.finalize()))
            }
            Self::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(content);
                Some(format!("{}", hasher.finalize()))
            }
        }
    }
}

/// Permission application strategies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PermissionStrategy {
    #[default]
    Standard,
    ReadOnly,
    Preserve,
    Owned,
}

/// Result of permission resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionResolution {
    /// The mode bits from the archive (if present)
    pub archive_mode: Option<u32>,
    /// The resolved PermissionMode to apply
    pub resolved: PermissionMode,
}

impl PermissionStrategy {
    /// Resolve permissions (pure function).
    pub fn resolve(self, mode: Option<u32>) -> PermissionResolution {
        let resolved = match self {
            Self::Standard => {
                if let Some(m) = mode {
                    if m & 0o111 != 0 {
                        PermissionMode::Custom(m)
                    } else {
                        PermissionMode::Custom(m | 0o644)
                    }
                } else {
                    PermissionMode::Custom(0o644)
                }
            }
            Self::ReadOnly => PermissionMode::ReadOnly,
            Self::Preserve => {
                if let Some(m) = mode {
                    PermissionMode::Custom(m)
                } else {
                    PermissionMode::Inherit
                }
            }
            Self::Owned => PermissionMode::Custom(0o644),
        };

        PermissionResolution {
            archive_mode: mode,
            resolved,
        }
    }

    /// Apply permissions to path (impure).
    pub fn apply_to_path(&self, path: &Path, mode: Option<u32>) -> Result<()> {
        let resolution = self.resolve(mode);
        resolution.resolved.apply_to_path(path).map_err(Error::from)
    }
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
mod tests_strategy {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn none_strategy_returns_none() {
        let cursor = Cursor::new(b"hello");
        let result = HashStrategy::None.compute(cursor).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn sha256_compute() {
        let cursor = Cursor::new(b"hello world");
        let result = HashStrategy::Sha256.compute(cursor).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn blake3_compute() {
        let cursor = Cursor::new(b"hello world");
        let result = HashStrategy::Blake3.compute(cursor).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 64);
    }

    #[test]
    fn standard_strategy_executable() {
        let resolution = PermissionStrategy::Standard.resolve(Some(0o755));
        assert_eq!(resolution.archive_mode, Some(0o755));
        assert_eq!(resolution.resolved, PermissionMode::Custom(0o755));
    }

    #[test]
    fn standard_strategy_non_executable() {
        let resolution = PermissionStrategy::Standard.resolve(Some(0o644));
        assert_eq!(resolution.archive_mode, Some(0o644));
        assert_eq!(resolution.resolved, PermissionMode::Custom(0o644));
    }

    #[test]
    fn standard_strategy_no_mode() {
        let resolution = PermissionStrategy::Standard.resolve(None);
        assert_eq!(resolution.archive_mode, None);
        assert_eq!(resolution.resolved, PermissionMode::Custom(0o644));
    }

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
    fn extraction_options_default() {
        let options = ExtractOptions::default();
        assert_eq!(options.perm_strategy, PermissionStrategy::Standard);
        assert_eq!(options.hash_strategy, HashStrategy::None);
        assert_eq!(options.strip_components, 0);
        assert!(options.expected_total_bytes.is_none());
        assert!(options.on_progress.is_none());
    }

    #[test]
    fn extraction_options_builder_pattern() {
        let options = ExtractOptions::default()
            .permission_strategy(PermissionStrategy::ReadOnly)
            .hash_strategy(HashStrategy::Sha256)
            .strip_components(1)
            .expected_total_bytes(1024);

        assert_eq!(options.perm_strategy, PermissionStrategy::ReadOnly);
        assert_eq!(options.hash_strategy, HashStrategy::Sha256);
        assert_eq!(options.strip_components, 1);
        assert_eq!(options.expected_total_bytes, Some(1024));
    }

    #[test]
    fn extraction_options_on_progress_callback() {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        
        let options = ExtractOptions::default().on_progress(Arc::new(move |_| {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        }));

        let progress = Progress {
            bytes_processed: 50,
            total_bytes: Some(100),
            percentage: Some(50.0),
            current_file: Some(PathBuf::from("test.txt")),
        };

        (options.on_progress.as_ref().unwrap())(progress);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn progress_percentage_with_total() {
        let progress = Progress {
            bytes_processed: 50,
            total_bytes: Some(100),
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), Some(50.0));
    }

    #[test]
    fn progress_percentage_zero_total() {
        let progress = Progress {
            bytes_processed: 0,
            total_bytes: Some(0),
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), Some(0.0));
    }

    #[test]
    fn progress_percentage_none_total() {
        let progress = Progress {
            bytes_processed: 50,
            total_bytes: None,
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), None);
    }

    #[test]
    fn progress_percentage_full() {
        let progress = Progress {
            bytes_processed: 100,
            total_bytes: Some(100),
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), Some(100.0));
    }

    #[test]
    fn permission_strategy_variants() {
        use PermissionStrategy::*;
        let variants = [Standard, ReadOnly, Preserve, Owned];
        for (i, variant) in variants.iter().enumerate() {
            let mut options = ExtractOptions::default();
            match i {
                0 => options = options.permission_strategy(Standard),
                1 => options = options.permission_strategy(ReadOnly),
                2 => options = options.permission_strategy(Preserve),
                3 => options = options.permission_strategy(Owned),
                _ => unreachable!(),
            }
            assert_eq!(options.perm_strategy, *variant);
        }
    }

    #[test]
    fn hash_strategy_variants() {
        use HashStrategy::*;
        let variants = [None, Sha256, Blake3];
        for (i, variant) in variants.iter().enumerate() {
            let mut options = ExtractOptions::default();
            match i {
                0 => options = options.hash_strategy(None),
                1 => options = options.hash_strategy(Sha256),
                2 => options = options.hash_strategy(Blake3),
                _ => unreachable!(),
            }
            assert_eq!(options.hash_strategy, *variant);
        }
    }

    #[test]
    fn basic_path_sanitization() {
        let options = ExtractOptions::default();
        let result = options.sanitize_path("bin/tool", test_base_path()).unwrap();
        assert_eq!(result.original, Path::new("bin/tool"));
        assert!(result.resolved.starts_with(test_base_path()));
    }

    #[test]
    fn path_with_component_stripping() {
        let options = ExtractOptions::default().strip_components(1);
        let result = options.sanitize_path("tool-1.0/bin/tool", test_base_path()).unwrap();

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
        let result = options.sanitize_path(malicious_path, test_base_path());
        assert!(matches!(result, Err(Error::ZipSlip { .. })));
    }

    #[test]
    fn symlink_target_sanitization() {
        let options = ExtractOptions::default();
        let target = "../lib";
        let symlink_location = test_base_path().join("bin/mylink");
        let result = options.sanitize_symlink_target(
            target, symlink_location, test_base_path()
        ).unwrap();
        assert!(result.starts_with(test_base_path()));
    }

    #[test]
    fn symlink_absolute_path_rejected() {
        let options = ExtractOptions::default();
        let absolute_target = if cfg!(windows) { "C:\\etc\\passwd" } else { "/etc/passwd" };
        let symlink_location = test_base_path().join("bin/mylink");
        let result = options.sanitize_symlink_target(
            absolute_target, symlink_location, test_base_path()
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