use std::collections::HashMap;

use super::Result;
use crate::{block::*, blockchain::*, transaction::*};
use bincode::{deserialize, serialize};
use sled;

pub struct UTXOSet {
    pub blockchain: Blockchain,
}

impl UTXOSet {
    // 获取包含 未花费交易输出 的交易列表
    pub fn find_spendable_outputs(
        &self,
        pub_key_hash: &[u8],
        amount: i32,
    ) -> Result<(i32, HashMap<String, Vec<i32>>)> {
        let mut unspent_outputs: HashMap<String, Vec<i32>> = HashMap::new();
        let mut accumulated = 0;

        let db = sled::open("data/utxos")?;
        for kv in db.iter() {
            let (k, v) = kv?;
            let txid = String::from_utf8(k.to_vec())?;
            let outs: TXOutputs = deserialize(&v.to_vec())?;

            for out_idx in 0..outs.outputs.len() {
                if outs.outputs[out_idx].is_locked_with_key(pub_key_hash) && accumulated < amount {
                    accumulated += outs.outputs[out_idx].value;

                    match unspent_outputs.get_mut(&txid) {
                        Some(v) => v.push(out_idx as i32),
                        None => {
                            unspent_outputs.insert(txid.clone(), vec![out_idx as i32]);
                        }
                    }
                }
            }
        }

        Ok((accumulated, unspent_outputs))
    }

    // 通过 pub_key_hash 获取 未花费输出
    pub fn find_utxos(&self, pub_key_hash: &[u8]) -> Result<TXOutputs> {
        let mut utxos = TXOutputs {
            outputs: Vec::new(),
        };

        let db = sled::open("data/utxos")?;
        for kv in db.iter() {
            let (_, v) = kv?;
            let outs: TXOutputs = deserialize(&v.to_vec())?;

            for out in outs.outputs {
                if out.is_locked_with_key(pub_key_hash) {
                    utxos.outputs.push(out.clone());
                }
            }
        }

        Ok(utxos)
    }

    // 获取交易总数
    pub fn count_transactions(&self) -> Result<i32> {
        let mut counter = 0;

        let db = sled::open("data/utxos")?;
        for kv in db.iter() {
            kv?;
            counter += 1;
        }

        Ok(counter)
    }

    // 通过区块交易更新 utxo 集
    pub fn update(&self, block: &Block) -> Result<()> {
        let db = sled::open("data/utxos")?;

        for tx in block.get_transaction() {
            if !tx.is_coinbase() {
                for vin in &tx.vin {
                    let mut update_outputs = TXOutputs {
                        outputs: Vec::new(),
                    };

                    let outs: TXOutputs = deserialize(&db.get(&vin.txid)?.unwrap().to_vec())?;
                    for out_idx in 0..outs.outputs.len() {
                        if out_idx != vin.vout as usize {
                            update_outputs.outputs.push(outs.outputs[out_idx].clone());
                        }
                    }

                    if update_outputs.outputs.is_empty() {
                        db.remove(&vin.txid)?;
                    } else {
                        db.insert(vin.txid.as_bytes(), serialize(&update_outputs)?)?;
                    }
                }
            }

            let mut new_outputs = TXOutputs {
                outputs: Vec::new(),
            };
            for out in &tx.vout {
                new_outputs.outputs.push(out.clone());
            }

            db.insert(tx.id.as_bytes(), serialize(&new_outputs)?)?;
        }

        Ok(())
    }

    /**
     * @desc 重建数据库
     */
    pub fn reindex(&self) -> Result<()> {
        std::fs::remove_dir_all("data/utxos").ok();
        let db = sled::open("data/utxos")?;

        let utxos = self.blockchain.find_utxo();

        for (txid, outs) in utxos {
            db.insert(txid.as_bytes(), serialize(&outs)?)?;
        }

        Ok(())
    }
}
