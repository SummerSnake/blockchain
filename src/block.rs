use super::*;
use bincode::serialize;
use crypto::{digest::Digest, sha2::Sha256};
use std::time::SystemTime;

const TARGET_HEXS: usize = 4;

#[derive(Debug)]
pub struct Block {
    timestamp: u128,
    data: String,
    prev_block_hash: String,
    hash: String,
    nonce: i32, // 区块nonce, 为了防⽌交易重播，ETH节点要求每笔交易必须有⼀个nonce数值
}

impl Block {
    /**
     * @desc 新建区块
     */
    pub fn new(data: String, prev_block_hash: String) -> Result<Block> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();

        let mut block = Block {
            timestamp,
            data,
            prev_block_hash,
            hash: String::new(),
            nonce: 0,
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
     * @desc 执行算法
     */
    fn run_proof_of_work(&mut self) -> Result<()> {
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
     * @desc 获取需要被哈希的数据序列值
     */
    fn prepare_hash_data(&self) -> Result<Vec<u8>> {
        let content = (
            self.prev_block_hash.clone(),
            self.data.clone(),
            self.timestamp,
            TARGET_HEXS,
            self.nonce,
        );
        let bytes = serialize(&content)?;

        Ok(bytes)
    }
}
