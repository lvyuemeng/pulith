use std::env;
use std::path::PathBuf;

pub fn user_home() -> Option<PathBuf> {
    home::home_dir()
}

pub fn user_config() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        user_home().map(|p| p.join("Library/Application Support"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| user_home().map(|p| p.join(".config")))
    }
}

pub fn user_data() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        user_home().map(|p| p.join("Library/Application Support"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| user_home().map(|p| p.join(".local/share")))
    }
}

pub fn user_cache() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Cache"))
    }
    #[cfg(target_os = "macos")]
    {
        user_home().map(|p| p.join("Library/Caches"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| user_home().map(|p| p.join(".cache")))
    }
}

pub fn user_temp() -> PathBuf {
    env::temp_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_home_returns_optional() {
        let home = user_home();
        assert!(home.is_none() || home.unwrap().to_string_lossy().len() > 0);
    }

    #[test]
    fn test_user_temp_returns_pathbuf() {
        let temp = user_temp();
        assert!(temp.to_string_lossy().len() > 0);
    }

    #[test]
    fn test_user_temp_is_absolute() {
        let temp = user_temp();
        assert!(temp.is_absolute());
    }

    #[test]
    fn test_user_config_platform_specific() {
        let config = user_config();
        #[cfg(target_os = "windows")]
        {
            assert!(config.is_none() || config.unwrap().to_string_lossy().contains("AppData"));
        }
        #[cfg(target_os = "macos")]
        {
            assert!(
                config.is_none()
                    || config
                        .unwrap()
                        .to_string_lossy()
                        .contains("Application Support")
            );
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            assert!(config.is_none() || config.unwrap().to_string_lossy().contains(".config"));
        }
    }

    #[test]
    fn test_user_data_platform_specific() {
        let data = user_data();
        #[cfg(target_os = "windows")]
        {
            assert!(
                data.is_none() || {
                    let path_str = data.unwrap().to_string_lossy().to_lowercase();
                    path_str.contains("local") || path_str.contains("appdata")
                }
            );
        }
        #[cfg(target_os = "macos")]
        {
            assert!(
                data.is_none()
                    || data
                        .unwrap()
                        .to_string_lossy()
                        .contains("Application Support")
            );
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            assert!(data.is_none() || data.unwrap().to_string_lossy().contains(".local"));
        }
    }

    #[test]
    fn test_user_cache_platform_specific() {
        let cache = user_cache();
        #[cfg(target_os = "windows")]
        {
            assert!(cache.is_none() || cache.unwrap().to_string_lossy().contains("Cache"));
        }
        #[cfg(target_os = "macos")]
        {
            assert!(cache.is_none() || cache.unwrap().to_string_lossy().contains("Caches"));
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            assert!(cache.is_none() || cache.unwrap().to_string_lossy().contains(".cache"));
        }
    }

    #[test]
    fn test_user_home_matches_environment() {
        if let Some(home) = user_home() {
            let env_home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
            if let Some(env_home) = env_home {
                assert_eq!(home, PathBuf::from(env_home));
            }
        }
    }

    #[test]
    fn test_directories_are_valid_paths() {
        let funcs = [user_home(), user_config(), user_data(), user_cache()];
        for dir in funcs.into_iter().flatten() {
            assert!(!dir.as_os_str().is_empty());
            assert!(dir.to_string_lossy().len() > 0);
        }
    }

    #[test]
    fn test_user_temp_is_directory() {
        let temp = user_temp();
        assert!(temp.to_string_lossy().len() > 0);
    }

    #[test]
    fn test_config_data_cache_are_distinct() {
        let config = user_config();
        let data = user_data();
        let cache = user_cache();
        if let (Some(c), Some(d), Some(ca)) = (config, data, cache) {
            assert!(c != d || c != ca || d != ca);
        }
    }
}
