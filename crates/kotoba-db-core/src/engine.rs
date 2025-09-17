use anyhow::Result;
use super::types::{Block, Cid};

/// A trait for pluggable storage engines.
/// This defines the basic key-value interface that the database core uses.
#[async_trait::async_trait]
pub trait StorageEngine: Send + Sync {
    /// Puts a key-value pair into the store.
    async fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Gets a value from the store by its key.
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Deletes a key-value pair from the store.
    async fn delete(&mut self, key: &[u8]) -> Result<()>;

    /// Scans a range of keys with a given prefix.
    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}

impl dyn StorageEngine {
    /// Stores a content-addressed block in the database.
    /// Returns the CID of the stored block.
    pub async fn put_block(&mut self, block: &Block) -> Result<Cid> {
        let cid = block.cid()?;
        let bytes = block.to_bytes()?;
        self.put(&cid, &bytes).await?;
        Ok(cid)
    }

    /// Retrieves a content-addressed block from the database by its CID.
    pub async fn get_block(&self, cid: &Cid) -> Result<Option<Block>> {
        match self.get(cid).await? {
            Some(bytes) => {
                let block = Block::from_bytes(&bytes)?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }
}
