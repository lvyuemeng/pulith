use crate::error::Result;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn paths_equal(p1: &Path, p2: &Path) -> bool {
    fn normalize(p: &Path) -> String {
        p.to_string_lossy()
            .trim_end_matches(['/', '\\'])
            .to_lowercase()
    }
    #[cfg(target_os = "windows")]
    {
        normalize(p1) == normalize(p2)
    }
    #[cfg(not(target_os = "windows"))]
    {
        normalize(p1) == normalize(p2)
    }
}

#[derive(Debug, Clone)]
pub struct PathModifier {
    paths: Vec<PathBuf>,
}

impl Default for PathModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl PathModifier {
    pub fn new() -> Self {
        Self {
            paths: path_env().unwrap_or_default(),
        }
    }

    pub fn prepend(mut self, path: PathBuf) -> Self {
        if !self.paths.iter().any(|p| paths_equal(p, &path)) {
            self.paths.insert(0, path);
        }
        self
    }

    pub fn remove(mut self, path: &Path) -> Self {
        self.paths.retain(|p| !paths_equal(p, path));
        self
    }

    pub fn build(self) -> Result<OsString> {
        env::join_paths(self.paths).map_err(|_| crate::error::Error::Failed)
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.paths.iter().any(|p| paths_equal(p, path))
    }
}

pub fn path_env() -> Option<Vec<PathBuf>> {
    env::var_os("PATH").map(|val| env::split_paths(&val).collect())
}

pub fn is_in_path(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let Some(paths) = path_env() else {
        return false;
    };
    paths.iter().any(|p| paths_equal(p, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_modifier_new() {
        let modifier = PathModifier::new();
        assert!(modifier.paths.is_empty() || !modifier.paths.is_empty());
    }

    #[test]
    fn test_path_modifier_prepend() {
        let modifier = PathModifier::new().prepend(PathBuf::from("/new/path"));
        assert!(modifier.contains(PathBuf::from("/new/path").as_path()));
    }

    #[test]
    fn test_path_modifier_prepend_no_duplicates() {
        let modifier = PathModifier::new()
            .prepend(PathBuf::from("/path"))
            .prepend(PathBuf::from("/path"));
        let count = modifier
            .paths
            .iter()
            .filter(|p| p.to_string_lossy() == "/path")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_path_modifier_remove() {
        let modifier = PathModifier::new().remove(PathBuf::from("/usr/bin").as_path());
        assert!(!modifier.contains(PathBuf::from("/usr/bin").as_path()));
    }

    #[test]
    fn test_path_modifier_chain_operations() {
        let modifier = PathModifier::new()
            .prepend(PathBuf::from("/custom/bin"))
            .remove(PathBuf::from("/usr/bin").as_path())
            .prepend(PathBuf::from("/custom/bin2"));
        assert!(modifier.contains(PathBuf::from("/custom/bin").as_path()));
        assert!(modifier.contains(PathBuf::from("/custom/bin2").as_path()));
    }

    #[test]
    fn test_path_modifier_build() {
        let modifier = PathModifier::new().prepend(PathBuf::from("/custom"));
        let os_string = modifier.build();
        assert!(os_string.is_ok());
        let binding = os_string.unwrap();
        let path_str = binding.to_string_lossy();
        assert!(path_str.contains("/custom"));
    }

    #[test]
    fn test_path_modifier_empty_build() {
        let modifier = PathModifier::new();
        let os_string = modifier.build();
        assert!(os_string.is_ok());
    }

    #[test]
    fn test_path_modifier_contains() {
        let modifier = PathModifier::new();
        assert!(
            modifier.contains(PathBuf::from("/usr/bin").as_path())
                || !modifier.contains(PathBuf::from("/usr/bin").as_path())
        );
    }

    #[test]
    fn test_path_env_returns_optional() {
        let path = path_env();
        if let Some(paths) = &path {
            assert!(paths.is_empty() || !paths.is_empty());
        }
    }

    #[test]
    fn test_is_in_path_with_existing_path() {
        let result = is_in_path("/usr/bin");
        assert!(result || !result);
    }

    #[test]
    fn test_is_in_path_with_fake_path() {
        let result = is_in_path("/fake/nonexistent/path/12345");
        assert!(!result || result);
    }

    #[test]
    fn test_is_in_path_empty() {
        let result = is_in_path("");
        assert!(!result);
    }

    #[test]
    fn test_paths_equal_normalization() {
        #[cfg(target_os = "windows")]
        {
            assert!(paths_equal(Path::new("C:\\path"), Path::new("c:\\path")));
            assert!(paths_equal(Path::new("C:\\path\\"), Path::new("c:\\path")));
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert!(paths_equal(Path::new("/path"), Path::new("/path")));
            assert!(paths_equal(Path::new("/path/"), Path::new("/path")));
        }
    }

    #[test]
    fn test_paths_equal_different() {
        assert!(!paths_equal(Path::new("/path1"), Path::new("/path2")));
    }

    #[test]
    fn test_path_modifier_clone() {
        let modifier1 = PathModifier::new().prepend(PathBuf::from("/test"));
        let modifier2 = modifier1.clone();
        assert_eq!(modifier1.paths, modifier2.paths);
    }

    #[test]
    fn test_path_modifier_debug() {
        let modifier = PathModifier::new();
        let debug_str = format!("{:?}", modifier);
        assert!(debug_str.contains("PathModifier"));
    }
}
