use super::*;
use crate::block::*;
use bincode::{deserialize, serialize};
use log::info;
use sled;

#[derive(Debug)]
pub struct Blockchain {
    tip: String,
    current_hash: String,
    db: sled::Db,
}

impl Blockchain {
    /**
     * @desc 新建区块链
     */
    pub fn new() -> Result<Blockchain> {
        info!("Creating new blockchain...");

        let db = sled::open("data/blocks")?;
        match db.get("LAST")? {
            Some(hash) => {
                info!("Found block database.");

                let last_hash = String::from_utf8(hash.to_vec())?;

                Ok(Blockchain {
                    tip: last_hash.clone(),
                    current_hash: last_hash,
                    db,
                })
            }
            None => {
                info!("Creating new block database...");

                // 创世块
                let genesis_block = Block::new(String::from("Genesis Block"), String::new())?;

                db.insert(genesis_block.get_hash(), serialize(&genesis_block)?)?;
                db.insert("LAST", genesis_block.get_hash().as_bytes())?;

                let bc = Blockchain {
                    tip: genesis_block.get_hash(),
                    current_hash: genesis_block.get_hash(),
                    db,
                };

                bc.db.flush()?;
                Ok(bc)
            }
        }
    }

    /**
     * @desc 添加区块
     */
    pub fn add_block(&mut self, data: String) -> Result<()> {
        info!("add new block to the chain");

        let last_hash = self.db.get("LAST")?.unwrap();
        let new_block = Block::new(data, String::from_utf8(last_hash.to_vec())?)?;

        self.db
            .insert(new_block.get_hash(), serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.db.flush()?;

        self.tip = new_block.get_hash();
        self.current_hash = new_block.get_hash();

        Ok(())
    }
}

impl Iterator for Blockchain {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encoded_block) = self.db.get(&self.current_hash) {
            return match encoded_block {
                Some(b) => {
                    if let Ok(block) = deserialize::<Block>(&b) {
                        self.current_hash = block.get_prev_hash();
                        Some(block)
                    } else {
                        None
                    }
                }
                None => None,
            };
        }

        None
    }
}
