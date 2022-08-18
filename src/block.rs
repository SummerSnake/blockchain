use super::*;
use bincode::serialize;
use crypto::{digest::Digest, sha2::Sha256};
use std::time::SystemTime;

#[derive(Debug)]
pub struct Block {
    timestamp: u128,
    data: String,
    prev_block_hash: String,
    hash: String,
}

impl Block {
    pub fn set_hash(&mut self) -> Result<()> {
        self.timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();

        let content = (self.data.clone(), self.timestamp);
        let bytes = serialize(&content)?;
        let mut hasher = Sha256::new();
        hasher.input(&bytes[..]);
        self.hash = hasher.result_str();
        Ok(())
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn new(data: String, prev_block_hash: String) -> Result<Block> {
        let mut block = Block {
            timestamp: 0,
            data,
            prev_block_hash,
            hash: String::new(),
        };

        block.set_hash()?;
        Ok(block)
    }
}
