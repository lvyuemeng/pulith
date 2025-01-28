pub mod backend_reg;
pub mod tool_reg;

use anyhow::{Result, bail};
use backend_reg::BACKEND_REG;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::SystemTime};
use tokio::fs::rename;
use tool_reg::TOOL_REG;

use crate::utils::task_pool::POOL;

pub trait Cache {
    fn load() -> Result<Self>;
    fn save(&self) -> Result<()>;
    fn locate() -> Result<PathBuf> {
        Ok(PathBuf::new())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Reg<T: Serialize + Default> {
    #[serde(skip)]
    dirty: bool,
    #[serde(skip)]
    last_hash: Option<Vec<u8>>,
    reg: T,
}

impl<T: Serialize + Default> Default for Reg<T> {
    fn default() -> Self {
        Self {
            dirty: false,
            last_hash: None,
            reg: T::default(),
        }
    }
}

impl<T> Cache for Reg<T> {
    fn load() -> Result<Self> {
        let path: PathBuf = Cache::<Self>::locate()?;
        if !path.exists() {
            return Ok(Reg::default());
        }

        // async
        POOL.block_on(async move {
            let mut file = File::open(&path).await?;
            let mut hasher = Sha256::new();
            let mut reader = BufReader::new(file);
            let mut content = Vec::new();

            io::copy(&mut reader, &mut content)?;
            hasher.update(&content);

            let reg = bincode::deserialize_from(&reader)?;
            Ok(reg)
        })
    }

    fn save(&self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let path: PathBuf = Cache::<Self>::locate()?;
        let tmp_path = path.with_extension("tmp");

        if let Some(cur_hash) = self.last_hash {
            let hash = POOL.block_on(async move {
                let mut file = File::open(&path).await?;
                let mut reader = BufReader::new(file);
                let mut hasher = Sha256::new();
                let mut content = Vec::new();
                io::copy(&mut reader, &mut content)?;

                hasher.update(&content);

                hasher.finalize().as_slice()
            });
            if hash != cur_hash {
                bail!("file changed externally")
            }
        }

        {
            POOL.block_on(async move {
                let mut file: File = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(&tmp_path)?;
                file.set_len(0)?;

                let data = bincode::serialize(self)?;
                file.write_all(&data)?;
                file.sync_all()?;
                rename(&tmp_path, &path)?;
                self.dirty = false;

                let mut hasher = Sha256::new();
                hasher.update(&data);
                self.last_hash = Some(hasher.finalize().to_vec());
            });
        }

        Ok(())
    }
}

pub struct SaveGuard {
    last_save: SystemTime,
}

impl SaveGuard {
    pub fn new() -> Self {
        Self {
            last_save: SystemTime::now(),
        }
    }
}

impl Drop for SaveGuard {
    fn drop(&mut self) {
        let _ = TOOL_REG.save();
        let _ = BACKEND_REG.save();
    }
}
