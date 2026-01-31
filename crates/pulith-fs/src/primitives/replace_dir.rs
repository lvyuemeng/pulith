use crate::{Error, Result};
use std::path::Path;

pub struct Options {
    pub retry_count: u32,
    pub retry_delay: std::time::Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            retry_count: 5,
            retry_delay: std::time::Duration::from_millis(100),
        }
    }
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
    pub fn retry_delay(mut self, delay: std::time::Duration) -> Self {
        self.retry_delay = delay;
        self
    }
}

pub fn replace_dir(src: impl AsRef<Path>, dest: impl AsRef<Path>, options: Options) -> Result<()> {
    let src = src.as_ref();
    let dest = dest.as_ref();

    #[cfg(unix)]
    {
        std::fs::rename(src, dest).map_err(|e| Error::ReplaceDir {
            path: dest.to_path_buf(),
            source: e,
        })
    }

    #[cfg(windows)]
    {
        use std::thread;
        let mut attempts = 0;
        loop {
            if dest.exists()
                && let Err(e) = std::fs::remove_dir_all(dest) {
                    attempts += 1;
                    if attempts >= options.retry_count {
                        return Err(Error::ReplaceDir {
                            path: dest.to_path_buf(),
                            source: e,
                        });
                    }
                    thread::sleep(options.retry_delay * attempts);
                    continue;
                }

            match std::fs::rename(src, dest) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts >= options.retry_count {
                        return Err(Error::ReplaceDir {
                            path: dest.to_path_buf(),
                            source: e,
                        });
                    }
                    thread::sleep(options.retry_delay * attempts);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_replace_dir() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("file.txt"), "data").unwrap();

        replace_dir(&src, &dest, Options::new()).unwrap();
        assert!(dest.exists());
        assert!(dest.join("file.txt").exists());
    }
}
