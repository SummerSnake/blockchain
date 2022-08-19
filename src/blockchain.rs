use super::*;
use crate::block::*;

#[derive(Debug)]
pub struct Blockchain {
    blocks: Vec<Block>,
}

impl Blockchain {
    /**
     * @desc 新建区块链
     */
    pub fn new() -> Self {
        // 创世块
        let genesis_block = Block::new(String::from("Genesis Block"), String::new()).unwrap();

        Blockchain {
            blocks: vec![genesis_block],
        }
    }

    /**
     * @desc 添加区块
     */
    pub fn add_block(&mut self, data: String) -> Result<()> {
        let prev = self.blocks.last().unwrap();
        let new_block = Block::new(data, prev.get_hash())?;
        self.blocks.push(new_block);
        Ok(())
    }
}
