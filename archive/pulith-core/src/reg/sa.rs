use postcard::to_vec;
use serde::{Serialize, de::DeserializeOwned};
use sled::IVec;
use std::path::Path;
use thiserror::Error;

#[derive(Debug)]
pub struct Cache<T> {
    db: sled::Db,
    _marker: std::marker::PhantomData<T>,
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Serialization Error {0}")]
    Serialization(#[from] postcard::Error),
    #[error("Database Error {0}")]
    Database(#[from] sled::Error),
    #[error("Transaction Error {0}")]
    Transaction(#[from] sled::transaction::TransactionError),
}

impl<T> Cache<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CacheError> {
        let db = sled::open(path)?;
        Ok(Self {
            db,
            _marker: std::marker::PhantomData,
        })
    }
    pub fn upsert<F>(&self, key: &str, f: F) -> Result<(), CacheError>
    where
        F: FnOnce(Option<T>) -> T,
    {
        let key = key.as_bytes();
        let val = "wadawdjwaljdwlakjdalw";
        let val2 = to_vec(&val).map_err(|e| CacheError::Serialization(e))?;
        self.db.insert(key, val);
        // self.db.insert(key, IVec::from(val2.as_ref()));
        self.db.insert(key, val2.as_ref());
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<T>, CacheError> {
        let key = key.as_bytes();
        let data = self.db.get(key)?;
        if let Some(data) = data {
            let data = postcard::from_bytes(&data).map_err(|e| CacheError::Serialization(e))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}
