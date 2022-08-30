use std::collections::HashMap;

use super::Result;
use crate::block::*;
use crate::transaction::*;
use bincode::{deserialize, serialize};
use failure::format_err;
use log::{debug, info};
use sled;

const GENESIS_COINBASE_DATA: &str = "The Rust is so hard, 淦~~";

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
            .expect("Must create a new block database first.");

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

        std::fs::remove_dir_all("data/blocks")?;
        let db = sled::open("data/blocks")?;

        debug!("Creating new block database...");

        let cbtx = Transaction::new_coinbase(address, String::from(GENESIS_COINBASE_DATA))?;
        let genesis_block = Block::new(vec![cbtx], String::new())?;
        db.insert(genesis_block.get_hash(), serialize(&genesis_block)?)?;
        db.insert("LAST", genesis_block.get_hash().as_bytes())?;

        let bc = Blockchain {
            tip: genesis_block.get_hash(),
            db,
        };
        bc.db.flush()?;

        Ok(bc)
    }

    /**
     * @desc 使用提供的交易挖掘新块
     */
    pub fn mine_block(&mut self, mut transactions: Vec<Transaction>) -> Result<Block> {
        info!("A new block.");

        for tx in &mut transactions {
            if !self.verify_transaction(tx)? {
                return Err(format_err!("ERROR: Invalid transaction."));
            }
        }

        let last_hash = self.db.get("LAST")?.unwrap();
        let new_block = Block::new(transactions, String::from_utf8(last_hash.to_vec())?)?;

        self.db
            .insert(new_block.get_hash(), serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.db.flush()?;

        self.tip = new_block.get_hash();

        Ok(new_block)
    }

    pub fn iter(&self) -> BlockchainIterator {
        BlockchainIterator {
            current_hash: self.tip.clone(),
            bc: &self,
        }
    }

    // 获取所有未花费交易输出
    pub fn find_utxo(&self) -> HashMap<String, TXOutputs> {
        let mut utxos: HashMap<String, TXOutputs> = HashMap::new();
        let mut spend_txos: HashMap<String, Vec<i32>> = HashMap::new();

        for block in self.iter() {
            for tx in block.get_transaction() {
                for index in 0..tx.vout.len() {
                    if let Some(ids) = spend_txos.get(&tx.id) {
                        if ids.contains(&(index as i32)) {
                            continue;
                        }
                    }

                    match utxos.get_mut(&tx.id) {
                        Some(v) => {
                            v.outputs.push(tx.vout[index].clone());
                        }
                        None => {
                            utxos.insert(
                                tx.id.clone(),
                                TXOutputs {
                                    outputs: vec![tx.vout[index].clone()],
                                },
                            );
                        }
                    }
                }

                if !tx.is_coinbase() {
                    for i in &tx.vin {
                        match spend_txos.get_mut(&i.txid) {
                            Some(v) => {
                                v.push(i.vout);
                            }
                            None => {
                                spend_txos.insert(i.txid.clone(), vec![i.vout]);
                            }
                        }
                    }
                }
            }
        }

        utxos
    }

    // 通过 id 获取交易
    pub fn find_transaction(&self, id: &str) -> Result<Transaction> {
        for b in self.iter() {
            for tx in b.get_transaction() {
                if tx.id == id {
                    return Ok(tx.clone());
                }
            }
        }

        Err(format_err!("Transaction is not found."))
    }

    // 验证交易签名
    pub fn verify_transaction(&self, tx: &mut Transaction) -> Result<bool> {
        if tx.is_coinbase() {
            return Ok(true);
        }

        let prev_txs = self.get_prev_txs(tx)?;
        tx.verify(prev_txs)
    }

    // 对交易的输入进行签名
    pub fn sign_transaction(&self, tx: &mut Transaction, private_key: &[u8]) -> Result<()> {
        let prev_txs = self.get_prev_txs(tx)?;
        tx.sign(private_key, prev_txs)?;
        Ok(())
    }

    // 获取前一笔交易
    fn get_prev_txs(&self, tx: &Transaction) -> Result<HashMap<String, Transaction>> {
        let mut prev_txs = HashMap::new();

        for vin in &tx.vin {
            let prev_tx = self.find_transaction(&vin.txid)?;
            prev_txs.insert(prev_tx.id.clone(), prev_tx);
        }

        Ok(prev_txs)
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
