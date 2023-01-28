//! This module implements the blockchain.
//! 
//! You need to implement the `Blockchain` struct and its methods.

use crate::block::Block;
use crate::crypto::hash::{H256, Hashable};
use std::collections::HashMap;

pub struct Blockchain {
    hash_to_block: HashMap<H256, Block>,
    hashvec: Vec<H256>,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let mut hash_to_block = HashMap::new();
        let key = Block::genesis().hash();
        hash_to_block.insert(key, Block::genesis());
        Blockchain {
            hash_to_block: hash_to_block,
            hashvec: vec![key.clone()],
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let newhash = block.hash();
        self.hash_to_block.insert(newhash, block.clone());
        //get block's parent
        let parent = block.header.parent;
        if self.tip() == parent{
            self.hashvec.push(newhash);
        } else{
            let mut total_length = 1;
            let mut curr_block = parent;
            while let Some(b) = self.hash_to_block.get(&curr_block){
                curr_block = b.header.parent;
                total_length += 1;
            }
            let mut longest_chain_length = total_length;
            println!("total length: {}", total_length);
            println!("hash vector length: {}", self.hashvec.len());
            if total_length > self.hashvec.len(){
                print!("here");
                //update hashvec
                while total_length > self.hashvec.len(){
                    self.hashvec.pop();
                    total_length = total_length - 1;
                }
                let mut movingparent = block.header.parent;
                while longest_chain_length > self.hashvec.len(){
                    if movingparent == self.tip(){
                        self.hashvec.push(movingparent);
                        self.hashvec.push(newhash);
                        longest_chain_length +=2;
                    }
                    else{
                        if let Some(d) = self.hash_to_block.get(&movingparent){
                            movingparent = d.header.parent;
                        }
                    }

                    
                }

            }

        }

    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        return self.hashvec.last().unwrap().clone();
        //return self.hashvec[self.hashvec.len() -1];
    }

    /// Get the last block's hash of the longest chain
    #[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        unimplemented!()
    }
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

