use super::*;
use crate::block::*;

#[derive(Debug)]
pub struct Blockchain {
    blocks: Vec<Block>,
}

impl Blockchain {
    pub fn new() -> Self {
        let genesis_block = Block::new(String::from("Genesis Block"), String::new()).unwrap();

        Blockchain {
            blocks: vec![genesis_block],
        }
    }

    pub fn add_block(&mut self, data: String) -> Result<()> {
        let prev = self.blocks.last().unwrap();
        let new_block = Block::new(data, prev.get_hash())?;
        self.blocks.push(new_block);
        Ok(())
    }
}
