pub mod sa;

use crate::task_pool::POOL;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub trait Cache: Sized {
    fn load(root: &Path, id: &Path) -> Result<Self, RegError>;
    fn save(&mut self, root: &Path, id: &Path) -> Result<(), RegError>;
    fn locate(root: &Path, id: &Path) -> Option<PathBuf> {
        let path = root.join(id);

        if path.exists() { Some(path) } else { None }
    }
}

#[derive(Debug, Serialize)]
struct Reg<T: Serialize + Default + for<'de> Deserialize<'de>> {
    #[serde(skip)]
    dirty: bool,
    last_hash: Option<Vec<u8>>,
    storage: T,
}

#[derive(Debug)]
pub struct RegLoader<T: Serialize + Default + for<'de> Deserialize<'de>> {
    reg: Reg<T>,
    root: PathBuf,
    id: PathBuf,
}

#[derive(Debug, Error)]
pub enum RegError {
    #[error("File {root}/{id} not found")]
    NotFound { root: PathBuf, id: PathBuf },
    #[error("Last Hash mismatch current hash, Cache changed externally")]
    HashError,
    #[error(transparent)]
    IoError(#[from] tokio::io::Error),
    #[error(transparent)]
    SerdeError(#[from] bincode::Error),
}

// Q: due to drop impl, Reg<T> must impl Deserialize and collide with T trait bound of Deseralize.
impl<'de, T: Serialize + Default + DeserializeOwned> Deserialize<'de> for Reg<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper<T> {
            last_hash: Option<Vec<u8>>,
            reg: T,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(Reg {
            dirty: false,
            last_hash: helper.last_hash,
            storage: helper.reg,
        })
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> Default for Reg<T> {
    fn default() -> Self {
        Self {
            dirty: false,
            last_hash: None,
            storage: T::default(),
        }
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> Cache for Reg<T> {
    fn load(root: &Path, id: &Path) -> Result<Self, RegError> {
        let path = match Self::locate(root, id) {
            Some(path) => path,
            None => return Ok(Reg::default()),
        };

        POOL.block_on(async move {
            let mut file = File::open(&path).await?;
            let mut content = Vec::new();
            file.read_to_end(&mut content).await?;

            let mut hasher = Sha256::new();
            Digest::update(&mut hasher, &content);
            let hash = hasher.finalize().to_vec();

            let reg: Self = bincode::deserialize(&content)?;

            if let Some(ref last_hash) = reg.last_hash {
                if *last_hash != hash {
                    return Err(RegError::HashError);
                }
            }

            Ok(reg)
        })
    }

    fn save(&mut self, root: &Path, id: &Path) -> Result<(), RegError> {
        if !self.dirty {
            return Ok(());
        }
        let path = match Self::locate(root, id) {
            Some(path) => path,
            None => Err(RegError::NotFound {
                root: root.to_path_buf(),
                id: id.to_path_buf(),
            })?,
        };
        let tmp_path = path.with_extension("tmp");

        let data = bincode::serialize(&self)?;

        let hash = POOL.block_on(async move {
            let mut file: File = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&tmp_path)
                .await?;

            file.write_all(&data).await?;
            file.sync_all().await?;

            fs::rename(&tmp_path, &path).await?;
            Ok::<_, RegError>(Sha256::digest(&data).to_vec())
        })?;

        self.last_hash = Some(hash);
        self.dirty = false;
        Ok(())
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> RegLoader<T> {
    pub fn load(root: &Path, id: &Path) -> Result<Self, RegError> {
        Ok(Self {
            reg: Reg::load(root, id)?,
            root: root.to_path_buf(),
            id: id.to_path_buf(),
        })
    }

    pub fn save(&mut self) -> Result<(), RegError> {
        let root = &self.root;
        let id = &self.id;
        self.reg.save(root, id)
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> Drop for RegLoader<T> {
    fn drop(&mut self) {
        let _ = self.save();
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> Deref for RegLoader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.reg.storage
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> DerefMut for RegLoader<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reg.storage
    }
}
