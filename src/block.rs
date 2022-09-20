use std::time::SystemTime;

use super::Result;
use crate::transaction::*;
use bincode::serialize;
use crypto::{digest::Digest, sha2::Sha256};
use log::info;
use merkle_cbt::merkle_tree::{Merge, CBMT};
use serde::{Deserialize, Serialize};

const TARGET_HEXS: usize = 4;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    timestamp: u128,
    transactions: Vec<Transaction>,
    prev_block_hash: String,
    hash: String,
    nonce: i32,
    height: i32,
}

impl Block {
    /**
     * @desc 新建区块
     */
    pub fn new(
        transactions: Vec<Transaction>,
        prev_block_hash: String,
        height: i32,
    ) -> Result<Block> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();

        let mut block = Block {
            timestamp,
            transactions,
            prev_block_hash,
            hash: String::new(),
            nonce: 0,
            height,
        };

        block.run_proof_of_work()?;
        Ok(block)
    }

    /**
     * @desc 获取区块 hash
     */
    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    /**
     * @desc 获取前一个区块 hash
     */
    pub fn get_prev_hash(&self) -> String {
        self.prev_block_hash.clone()
    }

    /**
     * @desc 获取交易记录
     */
    pub fn get_transaction(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    /**
     * @desc 获取区块高度(当前区块在区块链中和创世区块之间的块数)
     */
    pub fn get_height(&self) -> i32 {
        self.height
    }

    /**
     * @desc 执行算法
     */
    fn run_proof_of_work(&mut self) -> Result<()> {
        info!("Mining the block.");

        while !self.validate()? {
            self.nonce += 1;
        }

        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        self.hash = hasher.result_str();

        Ok(())
    }

    /**
     * @desc 判断当前的哈希值是否满足要求
     */
    fn validate(&self) -> Result<bool> {
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        let mut vec_tmp = Vec::new();
        vec_tmp.resize(TARGET_HEXS, '0' as u8);

        Ok(&hasher.result_str()[0..TARGET_HEXS] == String::from_utf8(vec_tmp)?)
    }

    /**
     * @desc 将交易转换成 Merkle 树
     */
    fn hash_transactions(&self) -> Result<Vec<u8>> {
        let mut transactions = Vec::new();

        for tx in &self.transactions {
            transactions.push(tx.hash()?.as_bytes().to_owned());
        }
        let tree = CBMT::<Vec<u8>, MergeVu8>::build_merkle_tree(&transactions);

        Ok(tree.root())
    }

    /**
     * @desc 获取需要被哈希的数据序列值
     */
    fn prepare_hash_data(&self) -> Result<Vec<u8>> {
        let content = (
            self.prev_block_hash.clone(),
            self.hash_transactions()?,
            self.timestamp,
            TARGET_HEXS,
            self.nonce,
        );
        let bytes = serialize(&content)?;

        Ok(bytes)
    }
}

struct MergeVu8 {}

impl Merge for MergeVu8 {
    type Item = Vec<u8>;

    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut hasher = Sha256::new();
        let mut data: Vec<u8> = left.clone();
        data.append(&mut right.clone());
        hasher.input(&data);
        let mut res: [u8; 32] = [0; 32];
        hasher.result(&mut res);

        res.to_vec()
    }
}
