//! Effect abstractions for file system and command operations.
//!
////! This module provides unified trait definitions for I/O operations,
//! enabling testability and abstraction across the codebase.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ExecuteError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("command failed: {cmd}")]
    CommandFailed { cmd: String },
}

pub trait FileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError>;
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError>;
    fn copy(&self, src: &Path, dst: &Path) -> Result<(), FsError>;
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError>;
    fn remove_file(&self, path: &Path) -> Result<(), FsError>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError>;
    fn exists(&self, path: &Path) -> bool;
    fn link(&self, original: &Path, link: &Path) -> Result<(), FsError>;
    fn unlink(&self, path: &Path) -> Result<(), FsError>;

    #[cfg(unix)]
    fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), FsError>;
}

pub trait ReadFileSystem: FileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, FsError>;
}

pub trait CommandRunner {
    fn run(&self, cmd: &str, args: &[&str], env: &[(&str, &str)]) -> Result<(), ExecuteError>;
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

    fn copy(&self, src: &Path, dst: &Path) -> Result<(), FsError> {
        std::fs::copy(src, dst).map_err(FsError::Io)?;
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        std::fs::rename(from, to)?;
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        std::fs::remove_file(path).map_err(FsError::Io)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        std::fs::remove_dir_all(path).map_err(FsError::Io)
    }

    fn exists(&self, path: &Path) -> bool { path.exists() }

    fn link(&self, original: &Path, link: &Path) -> Result<(), FsError> {
        if original.is_dir() {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(original, link)
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_dir(original, link)
            }
        } else {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(original, link)
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(original, link)
            }
        }
        .map_err(FsError::Io)
    }

    fn unlink(&self, path: &Path) -> Result<(), FsError> {
        std::fs::remove_file(path).map_err(FsError::Io)
    }

    #[cfg(unix)]
    fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), FsError> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, PermissionsExt::from_mode(mode)).map_err(FsError::Io)
    }
}

impl ReadFileSystem for OsFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        std::fs::read_to_string(path).map_err(FsError::Io)
    }
}

pub struct OsCommandRunner;

impl CommandRunner for OsCommandRunner {
    fn run(&self, cmd: &str, args: &[&str], env: &[(&str, &str)]) -> Result<(), ExecuteError> {
        let mut command = std::process::Command::new(cmd);
        command.args(args);
        for (k, v) in env {
            command.env(k, v);
        }
        let status = command.status().map_err(ExecuteError::Io)?;
        if status.success() {
            Ok(())
        } else {
            Err(ExecuteError::CommandFailed {
                cmd: cmd.to_string(),
            })
        }
    }
}

#[cfg(test)]
pub struct MemFileSystem {
    files: std::cell::RefCell<std::collections::HashMap<PathBuf, Vec<u8>>>,
}

#[cfg(test)]
impl MemFileSystem {
    pub fn new() -> Self {
        Self {
            files: std::cell::RefCell::new(std::collections::HashMap::new()),
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

    fn copy(&self, src: &Path, dst: &Path) -> Result<(), FsError> {
        let content = self
            .files
            .borrow()
            .get(&src.to_path_buf())
            .cloned()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "file not found").into()
            })?;
        self.files.borrow_mut().insert(dst.to_path_buf(), content);
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let content = self
            .files
            .borrow_mut()
            .remove(&from.to_path_buf())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "file not found").into()
            })?;
        self.files.borrow_mut().insert(to.to_path_buf(), content);
        Ok(())
    }

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

    fn link(&self, _original: &Path, _link: &Path) -> Result<(), FsError> { Ok(()) }

    fn unlink(&self, path: &Path) -> Result<(), FsError> {
        self.files.borrow_mut().remove(&path.to_path_buf());
        Ok(())
    }

    #[cfg(unix)]
    fn set_permissions(&self, _path: &Path, _mode: u32) -> Result<(), FsError> {
        Ok::<(), FsError>(())
    }
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
pub struct TestCommandRunner;

#[cfg(test)]
impl CommandRunner for TestCommandRunner {
    fn run(&self, _cmd: &str, _args: &[&str], _env: &[(&str, &str)]) -> Result<(), ExecuteError> {
        Ok(())
    }
}
