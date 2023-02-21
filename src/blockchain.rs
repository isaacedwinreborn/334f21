//! This module implements the blockchain.
//! 
//! You need to implement the `Blockchain` struct and its methods.

use crate::block::Block;
use crate::crypto::hash::{H256, Hashable};
use std::collections::HashMap;

pub struct Blockchain {
    hash_to_block: HashMap<H256, Block>,
    hashvec: Vec<H256>,
    hash_to_height: HashMap<H256, u32>,
    tip: H256,
    difficulty: H256,
    orphan_buffer: HashMap<H256, Vec<Block>>,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let mut hash_to_block = HashMap::new();
        let key = Block::genesis().hash();
        hash_to_block.insert(key, Block::genesis());
        let mut hash_to_height = HashMap:: new();
        hash_to_height.insert(key, 0);
        let mut orphan_hash = HashMap::new();
        Blockchain {
            hash_to_block: hash_to_block,
            hashvec: vec![key.clone()],
            hash_to_height: hash_to_height,
            tip: key,
            difficulty: Block::genesis().header.difficulty,
            orphan_buffer: orphan_hash,
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let newhash = block.hash();
        self.hash_to_block.insert(newhash, block.clone());
        self.hashvec.push(newhash);
        let curr_height = self.hash_to_height.get(&self.tip).unwrap();
        //get block's parent
        let parent = block.header.parent;
        let parent_height = self.hash_to_height.get(&parent);
        if (parent_height.unwrap() + 1) > *curr_height {
            self.tip = newhash;
        }
        self.hash_to_height.insert(newhash, parent_height.unwrap() + 1);
        let count = self.hash_to_block.len();
        print!("{} blocks inserted in blockchain \n", count);

    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        return self.tip;
        //return self.hashvec[self.hashvec.len() -1];
    }

    /// Get the last block's hash of the longest chain
    //#[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut longest_chain = Vec::new();
        let mut curr_hash = self.tip;
        while let Some(b) = self.hash_to_block.get(&curr_hash){
            longest_chain.push(curr_hash);
            let parent = b.header.parent;
            curr_hash = parent;
        }
        return longest_chain;
    }

    pub fn get_difficulty(&self) -> H256 {
        return self.difficulty;
    }
    pub fn get_block(&self, hash: &H256) -> &Block {
        self.hash_to_block.get(hash).unwrap()
    }

    pub fn insert_block_to_orphan_buffer(&mut self, block: &Block){
        let key = block.header.parent;
        if self.orphan_buffer.contains_key(&key){
            let vec = self.orphan_buffer.get_mut(&key).unwrap();
            vec.push(block.clone());
        }else{
            self.orphan_buffer.insert(key, vec![block.clone()]);
        }
    }

    pub fn update_orphan_buffer_and_blockchain(&mut self, block: &Block){
        let hash = block.hash();
        if self.orphan_buffer.contains_key(&hash){
            let removed_vec = self.orphan_buffer.remove(&hash).unwrap();
            for block in removed_vec {
                self.insert(&block);
                self.update_orphan_buffer_and_blockchain(&block);
            }
        }
    }

    /*pub fn get_block_count(&self) -> usize{
        return self.hash_to_block.len();
    } */

}


#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::block::test::generate_random_block;
    use crate::crypto::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }

    #[test]
    fn mp1_insert_chain() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let mut block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
        for _ in 0..50 {
            let h = block.hash();
            block = generate_random_block(&h);
            blockchain.insert(&block);
            assert_eq!(blockchain.tip(), block.hash());
        }
    }

    #[test]
    fn mp1_insert_3_fork_and_back() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&block_1);
        assert_eq!(blockchain.tip(), block_1.hash());
        let block_2 = generate_random_block(&block_1.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());
        let block_3 = generate_random_block(&block_2.hash());
        blockchain.insert(&block_3);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_1 = generate_random_block(&block_2.hash());
        blockchain.insert(&fork_block_1);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_2 = generate_random_block(&fork_block_1.hash());
        blockchain.insert(&fork_block_2);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_4 = generate_random_block(&block_3.hash());
        blockchain.insert(&block_4);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_5 = generate_random_block(&block_4.hash());
        blockchain.insert(&block_5);
        assert_eq!(blockchain.tip(), block_5.hash());
    }

}

