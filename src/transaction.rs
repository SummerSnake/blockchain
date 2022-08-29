use std::collections::HashMap;

use super::Result;
use crate::blockchain::*;
use crate::utxo_set::UTXOSet;
use crate::wallets::*;
use bincode::serialize;
use bitcoincash_addr::Address;
use crypto::{digest::Digest, ed25519, sha2::Sha256};
use failure::format_err;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

const SUBSIDY: i32 = 10;

// 输入
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXInput {
    pub txid: String,
    pub vout: i32,
    pub signature: Vec<u8>,
    pub pub_key: Vec<u8>,
}

impl TXInput {
    pub fn uses_key(&self, pub_key_hash: &[u8]) -> bool {
        let mut pubkey_hash = self.pub_key.clone();
        hash_pub_key(&mut pubkey_hash);

        pubkey_hash == pub_key_hash
    }
}

// 输出
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutput {
    pub value: i32,
    pub pub_key_hash: Vec<u8>,
}

impl TXOutput {
    pub fn new(value: i32, address: String) -> Result<Self> {
        let mut txo = TXOutput {
            value,
            pub_key_hash: Vec::new(),
        };
        txo.lock(&address)?;

        Ok(txo)
    }

    pub fn is_locked_with_key(&self, pub_key_hash: &[u8]) -> bool {
        self.pub_key_hash == pub_key_hash
    }

    fn lock(&mut self, address: &str) -> Result<()> {
        let pub_key_hash = Address::decode(address).unwrap().body;
        debug!("lock: {}", address);
        self.pub_key_hash = pub_key_hash;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutputs {
    pub outputs: Vec<TXOutput>,
}

// 交易
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub vin: Vec<TXInput>,
    pub vout: Vec<TXOutput>,
}

impl Transaction {
    // 生成一笔新的交易
    pub fn new_utxo(from: &str, to: &str, amount: i32, utxo: &UTXOSet) -> Result<Transaction> {
        info!("New UTXO Transaction from: {} to: {}.", from, to);

        let wallets = Wallets::new()?;
        let wallet = match wallets.get_wallet(from) {
            Some(w) => w,
            None => return Err(format_err!("Wallet not found.")),
        };
        if let None = wallets.get_wallet(to) {
            return Err(format_err!("To wallet not found."));
        }

        let mut pub_key_hash = wallet.public_key.clone();
        hash_pub_key(&mut pub_key_hash);

        let acc_v = utxo.find_spendable_outputs(&pub_key_hash, amount)?;
        if acc_v.0 < amount {
            error!("Not Enough balance");

            return Err(format_err!(
                "Not Enough balance: current balance {}",
                acc_v.0
            ));
        }

        let mut vin = Vec::new();
        for tx in acc_v.1 {
            for out in tx.1 {
                let input = TXInput {
                    txid: tx.0.clone(),
                    vout: out,
                    signature: Vec::new(),
                    pub_key: wallet.public_key.clone(),
                };

                vin.push(input);
            }
        }

        let mut vout = vec![TXOutput::new(amount, to.to_string())?];
        if acc_v.0 > amount {
            vout.push(TXOutput::new(acc_v.0 - amount, from.to_string())?);
        }

        let mut tx = Transaction {
            id: String::new(),
            vin,
            vout,
        };
        tx.id = tx.hash()?;
        utxo.blockchain
            .sign_transaction(&mut tx, &wallet.secret_key)?;

        Ok(tx)
    }

    // 生成新币 - 矿工获得挖出新块的奖励
    pub fn new_coinbase(to: String, mut data: String) -> Result<Transaction> {
        info!("New coinbase Transaction to: {}", to);

        if data.is_empty() {
            data = format!("Reward to '{}'", to);
        }

        let wallets = Wallets::new()?;
        if let None = wallets.get_wallet(&to) {
            return Err(format_err!("coinbase wallet not found"));
        };

        let mut tx = Transaction {
            id: String::new(),
            vin: vec![TXInput {
                txid: String::new(),
                vout: -1,
                signature: Vec::new(),
                pub_key: Vec::from(data.as_bytes()),
            }],
            vout: vec![TXOutput::new(SUBSIDY, to)?],
        };
        tx.id = tx.hash()?;

        Ok(tx)
    }

    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.is_empty() && self.vin[0].vout == -1
    }

    pub fn verify(&mut self, prev_txs: HashMap<String, Transaction>) -> Result<bool> {
        if self.is_coinbase() {
            return Ok(true);
        }

        for vin in &self.vin {
            if prev_txs.get(&vin.txid).unwrap().id.is_empty() {
                return Err(format_err!("ERROR: Previous transaction is not correct."));
            }
        }

        let mut tx_copy = self.trim_copy();
        for in_id in 0..self.vin.len() {
            let prev_tx = prev_txs.get(&self.vin[in_id].txid).unwrap();
            tx_copy.vin[in_id].signature.clear();
            tx_copy.vin[in_id].pub_key = prev_tx.vout[self.vin[in_id].vout as usize]
                .pub_key_hash
                .clone();
            tx_copy.id = tx_copy.hash()?;
            tx_copy.vin[in_id].pub_key = Vec::new();

            if !ed25519::verify(
                &tx_copy.id.as_bytes(),
                &self.vin[in_id].pub_key,
                &self.vin[in_id].signature,
            ) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn sign(
        &mut self,
        private_key: &[u8],
        prev_txs: HashMap<String, Transaction>,
    ) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }

        for vin in &self.vin {
            if prev_txs.get(&vin.txid).unwrap().id.is_empty() {
                return Err(format_err!("ERROR: Previous transaction is not correct."));
            }
        }

        let mut tx_copy = self.trim_copy();
        for in_id in 0..tx_copy.vin.len() {
            let prev_tx = prev_txs.get(&tx_copy.vin[in_id].txid).unwrap();
            tx_copy.vin[in_id].signature.clear();
            tx_copy.vin[in_id].pub_key = prev_tx.vout[tx_copy.vin[in_id].vout as usize]
                .pub_key_hash
                .clone();
            tx_copy.id = tx_copy.hash()?;
            tx_copy.vin[in_id].pub_key = Vec::new();
            let signature = ed25519::signature(tx_copy.id.as_bytes(), private_key);
            self.vin[in_id].signature = signature.to_vec();
        }

        Ok(())
    }

    pub fn hash(&self) -> Result<String> {
        let mut copy = self.clone();
        copy.id = String::new();

        let data = serialize(&copy)?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);

        Ok(hasher.result_str())
    }

    fn trim_copy(&self) -> Transaction {
        let mut vin = Vec::new();
        let mut vout = Vec::new();

        for v in &self.vin {
            vin.push(TXInput {
                txid: v.txid.clone(),
                vout: v.vout.clone(),
                signature: Vec::new(),
                pub_key: Vec::new(),
            })
        }
        for v in &self.vout {
            vout.push(TXOutput {
                value: v.value,
                pub_key_hash: v.pub_key_hash.clone(),
            })
        }

        Transaction {
            id: self.id.clone(),
            vin,
            vout,
        }
    }
}
