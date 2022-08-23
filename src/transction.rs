use super::*;
use crate::blockchain::*;
use bincode::serialize;
use crypto::{digest::Digest, sha2::Sha256};
use failure::format_err;
use log::{error, info};
use serde::{Deserialize, Serialize};

const SUBSIDY: i32 = 10;

// 输入
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXInput {
    pub txid: String,
    pub vout: i32,
    pub script_sig: String,
}

impl TXInput {
    pub fn can_unlock_output_with(&self, unlocking_data: &str) -> bool {
        self.script_sig == unlocking_data
    }
}

// 输出
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutput {
    pub value: i32,
    pub script_pub_key: String,
}

impl TXOutput {
    pub fn can_be_unlock_with(&self, unlocking_data: &str) -> bool {
        self.script_pub_key == unlocking_data
    }
}

// 交易
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transction {
    pub id: String,
    pub vin: Vec<TXInput>,
    pub vout: Vec<TXOutput>,
}

impl Transction {
    // 生成一笔新的交易
    pub fn new_UTXO(from: &str, to: &str, amount: i32, bc: &Blockchain) -> Result<Transction> {
        info!("New UTXO Transaction from: {} to: {}.", from, to);

        let mut vin = Vec::new();
        let acc_v = bc.find_spendable_outputs(from, amount);

        if acc_v.0 < amount {
            error!("Not Enough balance");

            return Err(format_err!(
                "Not Enough balance: current balance {}",
                acc_v.0
            ));
        }

        for tx in acc_v.1 {
            for out in tx.1 {
                let input = TXInput {
                    txid: tx.0.clone(),
                    vout: out,
                    script_sig: String::from(from),
                };

                vin.push(input);
            }
        }

        let mut vout = vec![TXOutput {
            value: amount,
            script_pub_key: String::from(to),
        }];

        if acc_v.0 > amount {
            vout.push(TXOutput {
                value: acc_v.0 - amount,
                script_pub_key: String::from(from),
            });
        }

        let mut tx = Transction {
            id: String::new(),
            vin,
            vout,
        };

        tx.set_id()?;

        Ok(tx)
    }

    // 生成新币 - 矿工获得挖出新块的奖励
    pub fn new_coinbase(to: String, mut data: String) -> Result<Transction> {
        info!("New coinbase Transaction to: {}", to);

        if data == String::from("") {
            data += &format!("Reward to '{}'", to);
        }

        let mut tx = Transction {
            id: String::new(),
            vin: vec![TXInput {
                txid: String::new(),
                vout: -1,
                script_sig: data,
            }],
            vout: vec![TXOutput {
                value: SUBSIDY,
                script_pub_key: to,
            }],
        };

        tx.set_id()?;

        Ok(tx)
    }

    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.is_empty() && self.vin[0].vout == -1
    }

    fn set_id(&mut self) -> Result<()> {
        let mut hasher = Sha256::new();
        let data = serialize(self)?;
        hasher.input(&data);
        self.id = hasher.result_str();

        Ok(())
    }
}
