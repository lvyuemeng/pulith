use anyhow::{Result, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::SystemTime};
use tokio::fs::{rename, File, OpenOptions};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::utils::task_pool::POOL;

pub trait Cache:Sized {
    fn load() -> Result<Self>;
    fn save(&mut self) -> Result<()>;
    fn locate() ->Option<PathBuf>{
        None
    }
}

#[derive(Debug, Serialize)]
pub struct Reg<T: Serialize + Default + for<'de> Deserialize<'de>> {
    #[serde(skip)]
    dirty: bool,
    last_hash: Option<Vec<u8>>,
    reg: T,
}

// Q: due to drop impl, Reg<T> must impl Deserialize and collide with T trait bound of Deseralize.
impl<'de,T:Serialize+Default+DeserializeOwned> Deserialize<'de> for Reg<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        #[derive(Deserialize)]
        struct Helper<T> {
            last_hash: Option<Vec<u8>>,
            reg: T,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(Reg {
            dirty: false,
            last_hash: helper.last_hash,
            reg: helper.reg,
        })
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> Default for Reg<T> {
    fn default() -> Self {
        Self {
            dirty: false,
            last_hash: None,
            reg: T::default(),
        }
    }
}

impl<T: Serialize + Default + for<'de> Deserialize<'de>> Cache for Reg<T> {
    fn load() -> Result<Self> {
        let path = match Self::locate() {
            Some(path) => path,
            None =>return  Ok(Reg::default()),
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
                    bail!("Cache changed externally")
                }
            }
            
            Ok(reg)
        })
    }

    fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let path = match Self::locate() {
            Some(path) => path,None => bail!("cache file not found"),
        };
        let tmp_path = path.with_extension("tmp");

        let data = bincode::serialize(&self)?;

        let hash = POOL.block_on(async move {
            let mut file: File = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&tmp_path).await?;

            file.write_all(&data).await?;
            file.sync_all().await?;

            rename(&tmp_path, &path).await?;
            Ok::<_,anyhow::Error>(Sha256::digest(&data).to_vec())
        })?;

        self.last_hash = Some(hash);
        self.dirty = false;
        Ok(())
    }
}

impl<T:Serialize+Default+for<'de> Deserialize<'de>> Drop for Reg<T> {
    fn drop(&mut self) {
        let _ = self.save();
    }
}

