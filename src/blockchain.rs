use std::collections::HashMap;

use super::*;
use crate::block::*;
use crate::transction::*;
use bincode::{deserialize, serialize};
use log::{debug, info};
use sled;

const GENESIS_COINBASE_DATA: &str = "The Rust is so hard, Wuuu~~";

#[derive(Debug)]
pub struct Blockchain {
    tip: String,
    db: sled::Db,
}

pub struct BlockchainIterator<'a> {
    current_hash: String,
    bc: &'a Blockchain,
}

impl Blockchain {
    /**
     * @desc 创建区块
     */
    pub fn new() -> Result<Blockchain> {
        info!("Open blockchain...");

        let db = sled::open("data/blocks")?;
        let hash = db
            .get("LAST")?
            .expect("Must create a new block database first");

        info!("Found block database.");

        let last_hash = String::from_utf8(hash.to_vec())?;

        Ok(Blockchain {
            tip: last_hash.clone(),
            db,
        })
    }

    /**
     * @desc 创建区块链
     */
    pub fn create_blockchain(address: String) -> Result<Blockchain> {
        info!("Creating new blockchain.");

        let db = sled::open("data/blocks")?;

        debug!("Creating new block database...");

        let cb = Transction::new_coinbase(address, String::from(GENESIS_COINBASE_DATA))?;
        let genesis_block = Block::new(vec![cb], String::new())?;
        db.insert(genesis_block.get_hash(), serialize(&genesis_block)?)?;
        db.insert("LAST", genesis_block.get_hash().as_bytes())?;

        let bc = Blockchain {
            tip: genesis_block.get_hash(),
            db,
        };

        bc.db.flush();
        Ok(bc)
    }

    pub fn mine_block(&mut self, transactions: Vec<Transction>) -> Result<()> {
        info!("A new block.");

        let last_hash = self.db.get("LAST")?.unwrap();
        let new_block = Block::new(transactions, String::from_utf8(last_hash.to_vec())?)?;

        self.db
            .insert(new_block.get_hash(), serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.db.flush()?;
        self.tip = new_block.get_hash();

        Ok(())
    }

    pub fn iter(&self) -> BlockchainIterator {
        BlockchainIterator {
            current_hash: self.tip.clone(),
            bc: &self,
        }
    }

    pub fn find_UTXO(&self, address: &str) -> Vec<TXOutput> {
        let mut utxos = Vec::<TXOutput>::new();
        let unspend_TXs = self.find_unspend_transactions(address);

        for tx in unspend_TXs {
            for out in &tx.vout {
                if out.can_be_unlock_with(&address) {
                    utxos.push(out.clone());
                }
            }
        }

        utxos
    }

    pub fn find_spendable_outputs(
        &self,
        address: &str,
        amount: i32,
    ) -> (i32, HashMap<String, Vec<i32>>) {
        let mut unspent_outputs: HashMap<String, Vec<i32>> = HashMap::new();
        let mut accumulated = 0;
        let unspend_TXs = self.find_unspend_transactions(address);

        for tx in unspend_TXs {
            for index in 0..tx.vout.len() {
                if tx.vout[index].can_be_unlock_with(address) && accumulated < amount {
                    match unspent_outputs.get_mut(&tx.id) {
                        Some(v) => v.push(index as i32),
                        None => {
                            unspent_outputs.insert(tx.id.clone(), vec![index as i32]);
                        }
                    }

                    accumulated += tx.vout[index].value;

                    if accumulated >= amount {
                        return (accumulated, unspent_outputs);
                    }
                }
            }
        }

        (accumulated, unspent_outputs)
    }

    fn find_unspend_transactions(&self, address: &str) -> Vec<Transction> {
        let mut spent_TXOs: HashMap<String, Vec<i32>> = HashMap::new();
        let mut unspend_TXs: Vec<Transction> = Vec::new();

        for block in self.iter() {
            for tx in block.get_transaction() {
                for index in 0..tx.vout.len() {
                    if let Some(ids) = spent_TXOs.get(&tx.id) {
                        if ids.contains(&(index as i32)) {
                            continue;
                        }
                    }

                    if tx.vout[index].can_be_unlock_with(address) {
                        unspend_TXs.push(tx.to_owned())
                    }
                }

                if !tx.is_coinbase() {
                    for i in &tx.vin {
                        if i.can_unlock_output_with(address) {
                            match spent_TXOs.get_mut(&i.txid) {
                                Some(v) => {
                                    v.push(i.vout);
                                }
                                None => {
                                    spent_TXOs.insert(i.txid.clone(), vec![i.vout]);
                                }
                            }
                        }
                    }
                }
            }
        }

        unspend_TXs
    }
}

impl<'a> Iterator for BlockchainIterator<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encoded_block) = self.bc.db.get(&self.current_hash) {
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
