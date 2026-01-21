//! Effect abstractions for file system and network operations.
//!
//! This module provides unified trait definitions for I/O operations,
//! enabling testability and abstraction across the codebase.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
}

#[derive(Debug, Error)]
pub enum EnvironmentError {
    #[error("environment variable read failed: {0}")]
    VarRead(#[from] std::env::VarError),
    #[error("current directory read failed: {0}")]
    CurrentDir(#[from] std::io::Error),
}

pub trait FileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError>;
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError>;
    fn remove_file(&self, path: &Path) -> Result<(), FsError>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError>;
    fn exists(&self, path: &Path) -> bool;

    #[cfg(unix)]
    fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), FsError>;

    #[cfg(unix)]
    fn symlink(&self, original: &Path, link: &Path) -> Result<(), FsError>;
}

pub trait ReadFileSystem: FileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, FsError>;
}

#[async_trait::async_trait]
pub trait AsyncFileSystem {
    async fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;
    async fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError>;
    async fn create(&self, path: &Path) -> Result<tokio::fs::File, FsError>;
    async fn create_dir_all(&self, path: &Path) -> Result<(), FsError>;
    async fn remove_file(&self, path: &Path) -> Result<(), FsError>;
    async fn remove_dir_all(&self, path: &Path) -> Result<(), FsError>;
    async fn exists(&self, path: &Path) -> bool;

    #[cfg(unix)]
    async fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), FsError>;
}

#[async_trait::async_trait]
pub trait Network {
    async fn get(&self, url: &str) -> Result<bytes::Bytes, NetworkError>;
}

pub trait Environment {
    fn var(&self, name: &str) -> Option<String>;
    fn current_dir(&self) -> Option<PathBuf>;
}

pub struct OsFileSystem;

impl FileSystem for OsFileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        std::fs::read(path).map_err(FsError::Io)
    }

    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        std::fs::write(path, content).map_err(FsError::Io)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        std::fs::create_dir_all(path).map_err(FsError::Io)
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        std::fs::remove_file(path).map_err(FsError::Io)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        std::fs::remove_dir_all(path).map_err(FsError::Io)
    }

    fn exists(&self, path: &Path) -> bool { path.exists() }

    #[cfg(unix)]
    fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), FsError> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, PermissionsExt::from_mode(mode)).map_err(FsError::Io)
    }

    #[cfg(unix)]
    fn symlink(&self, original: &Path, link: &Path) -> Result<(), FsError> {
        std::os::unix::fs::symlink(original, link).map_err(FsError::Io)
    }
}

impl ReadFileSystem for OsFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        std::fs::read_to_string(path).map_err(FsError::Io)
    }
}

pub struct TokioFileSystem;

#[async_trait::async_trait]
impl AsyncFileSystem for TokioFileSystem {
    async fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        tokio::fs::read(path).await.map_err(FsError::Io)
    }

    async fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        tokio::fs::write(path, content).await.map_err(FsError::Io)
    }

    async fn create(&self, path: &Path) -> Result<tokio::fs::File, FsError> {
        tokio::fs::File::create(path).await.map_err(FsError::Io)
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        tokio::fs::create_dir_all(path).await.map_err(FsError::Io)
    }

    async fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        tokio::fs::remove_file(path).await.map_err(FsError::Io)
    }

    async fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        tokio::fs::remove_dir_all(path).await.map_err(FsError::Io)
    }

    async fn exists(&self, path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    #[cfg(unix)]
    async fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), FsError> {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, PermissionsExt::from_mode(mode))
            .await
            .map_err(FsError::Io)
    }
}

pub struct HttpNetwork;

#[async_trait::async_trait]
impl Network for HttpNetwork {
    async fn get(&self, url: &str) -> Result<bytes::Bytes, NetworkError> {
        let client = reqwest::Client::new();
        client
            .get(url)
            .send()
            .await
            .map_err(NetworkError::Reqwest)?
            .bytes()
            .await
            .map_err(NetworkError::Reqwest)
    }
}

pub struct OsEnvironment;

impl Environment for OsEnvironment {
    fn var(&self, name: &str) -> Option<String> { std::env::var(name).ok() }
    fn current_dir(&self) -> Option<PathBuf> { std::env::current_dir().ok() }
}

#[cfg(test)]
pub struct MemFileSystem {
    files: RefCell<std::collections::HashMap<PathBuf, Vec<u8>>>,
}

#[cfg(test)]
impl MemFileSystem {
    pub fn new() -> Self {
        Self {
            files: RefCell::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(test)]
impl FileSystem for MemFileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.files
            .borrow()
            .get(&path.to_path_buf())
            .cloned()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "file not found").into()
            })
    }

    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), content.to_vec());
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> Result<(), FsError> { Ok(()) }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.files
            .borrow_mut()
            .remove(&path.to_path_buf())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "file not found").into()
            })?;
        Ok(())
    }

    fn remove_dir_all(&self, _path: &Path) -> Result<(), FsError> { Ok(()) }

    fn exists(&self, path: &Path) -> bool { self.files.borrow().contains_key(&path.to_path_buf()) }

    #[cfg(unix)]
    fn set_permissions(&self, _path: &Path, _mode: u32) -> Result<(), FsError> { Ok(()) }

    #[cfg(unix)]
    fn symlink(&self, _original: &Path, _link: &Path) -> Result<(), FsError> { Ok(()) }
}

#[cfg(test)]
impl ReadFileSystem for MemFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        self.files
            .borrow()
            .get(&path.to_path_buf())
            .cloned()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "file not found").into()
            })
            .and_then(|bytes| {
                String::from_utf8(bytes).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid UTF-8").into()
                })
            })
    }
}

#[cfg(test)]
pub struct MemEnvironment {
    vars: std::collections::HashMap<String, String>,
    cwd:  Option<PathBuf>,
}

#[cfg(test)]
impl MemEnvironment {
    pub fn new() -> Self {
        Self {
            vars: std::collections::HashMap::new(),
            cwd:  Some(PathBuf::from("/cwd")),
        }
    }

    pub fn with_var(mut self, name: &str, value: &str) -> Self {
        self.vars.insert(name.to_string(), value.to_string());
        self
    }

    pub fn with_no_cwd(mut self) -> Self {
        self.cwd = None;
        self
    }
}

#[cfg(test)]
impl Environment for MemEnvironment {
    fn var(&self, name: &str) -> Option<String> { self.vars.get(name).cloned() }
    fn current_dir(&self) -> Option<PathBuf> { self.cwd.clone() }
}
