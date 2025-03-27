use std::path::Path;

use thiserror::Error;

use super::BackendType;

pub mod backend_reg;
pub mod tool_reg;

#[derive(Debug, Error)]
pub enum RegError {
    #[error("Serialization Error {0}")]
    Serialization(#[from] postcard::Error),
    #[error("Database Error {0}")]
    Database(#[from] sled::Error),
    #[error("Transaction Error {0}")]
    Transaction(#[from] sled::transaction::TransactionError),
}

pub struct Inventory {
    db: sled::Db,
}

pub struct DbKeys;

impl Inventory {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, RegError> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn get_snap(&self, bk: &BackendType) -> Result<Option<Snap, RegError>> {
        let key = DbKeys::bk_snap_key(bk);
        let data = self.db.get(key)?;
        if let Some(data) = data {
            let data: Snap = postcard::from_bytes(&data).map_err(|e| RegError::Serialization(e))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_snap(&self, bk: &BackendType, snap: &Snap) -> Result<(), RegError> {
        let key = DbKeys::bk_snap_key(bk);
        let val = postcard::to_vec(snap).map_err(|e| RegError::Serialization(e))?;
        self.db.insert(key, val)?;
        Ok(())
    }

    pub fn get_tool(&self, bk: &BackendType, tool: &str) -> Result<Option<ToolStatus, RegError>> {
        let key = DbKeys::tool_key(bk, tool);
        let data = self.db.get(key)?;
        if let Some(data) = data {
            let data: ToolStatus =
                postcard::from_bytes(&data).map_err(|e| RegError::Serialization(e))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_tool(
        &self,
        bk: &BackendType,
        tool: &str,
        tool_status: &ToolStatus,
    ) -> Result<(), RegError> {
        let key = DbKeys::tool_key(bk, tool);
        let val = postcard::to_vec(tool_status).map_err(|e| RegError::Serialization(e))?;
        self.db.insert(key, val)?;
        Ok(())
    }

    pub fn get_tools(&self, bk: Option<&BackendType>) -> Result<Vec<ToolStatus>, RegError> {
        let key_prefix = DbKeys::tool_key_prefix(bk);
        let tool_iter = self.db.scan_prefix(&key_prefix);
        let mut tools = Vec::new();
        for entry in tool_iter {
            let (_, val) = entry?;
            let tool = postcard::from_bytes(&val).map_err(|e| RegError::Serialization(e))?;
            tools.push(tool);
        }

        Ok(tools)
    }
}

impl DbKeys {
    pub fn bk_snap_key(bk: &BackendType) -> Vec<u8> {
        format!("bk:snap:{}", hex::encode(bk)).into_bytes()
    }

    pub fn tool_key_prefix(bk: Option<&BackendType>) -> Vec<u8> {
        if let Some(bk) = bk {
            return format!("bk:tool:{}:", hex::encode(bk)).into_bytes();
        }
        // for all
        format!("bk:tool:").into_bytes()
    }

    pub fn tool_key(bk: &BackendType, tool: &str) -> Vec<u8> {
        format!("bk:tool:{}:{}", hex::encode(bk), hex::encode(tool)).into_bytes()
    }
}
