use super::hash::{Hashable, H256};
use ring::digest;

#[derive(Debug, Default, Clone)]
struct MerkleTreeNode {
    left: Option<Box<MerkleTreeNode>>,
    right: Option<Box<MerkleTreeNode>>,
    hash: H256,
}

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    root: MerkleTreeNode,
    level_count: usize, // how many levels the tree has
}

/// Given the hash of the left and right nodes, compute the hash of the parent node.
fn hash_children(left: &H256, right: &H256) -> H256 {
    let concat = [left.as_ref(), right.as_ref()].concat();
    let name_hash = digest::digest(&digest::SHA256, &concat);
    return name_hash.into();
}

/// Duplicate the last node in `nodes` to make its length even.
fn duplicate_last_node(nodes: &mut Vec<Option<MerkleTreeNode>>) {
    nodes.push(nodes[nodes.len() - 1].clone());
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        assert!(!data.is_empty());

        // create the leaf nodes:
        let mut curr_level: Vec<Option<MerkleTreeNode>> = Vec::new();
        for item in data {
            curr_level.push(Some(MerkleTreeNode { hash: item.hash(), left: None, right: None }));
        }
        let mut level_count = 1;
        
        // create the upper levels of the tree:
        while curr_level.len() > 1 {
            // Whenever a level of the tree has odd number of nodes, duplicate the last node to make the number even:
            if curr_level.len() % 2 == 1 {
                duplicate_last_node(&mut curr_level);
            }
            assert_eq!(curr_level.len() % 2, 0); // make sure we now have even number of nodes.

            let mut next_level: Vec<Option<MerkleTreeNode>> = Vec::new();
            for i in 0..curr_level.len() / 2 {
                let left = curr_level[i * 2].take().unwrap();
                let right = curr_level[i * 2 + 1].take().unwrap();
                let hash = hash_children(&left.hash, &right.hash); 
                next_level.push(Some(MerkleTreeNode { hash: hash, left: Some(Box::new(left)), right: Some(Box::new(right)) }));
            }
            curr_level = next_level;
            level_count += 1;
        }
        MerkleTree {
            root: curr_level[0].take().unwrap(),
            level_count: level_count,
        }
    }

    pub fn root(&self) -> H256 {
        return self.root.hash;
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {

        // want to get the starting node of the proof
        let mut proof = Vec::new();
        let mut current_node = &self.root;
        let mut number_of_nodes = 1 << (self.level_count - 1);
        let mut count = self.level_count;
        let mut start = 0;
        let mut end = number_of_nodes;

        while count > 1 {
            if index < (start + end) / 2 {
                proof.push(current_node.right.as_ref().unwrap().hash);
                current_node = current_node.left.as_ref().unwrap();
                end = (start + end) / 2 - 1;
            }
            else{
                proof.push(current_node.left.as_ref().unwrap().hash);
                current_node = current_node.right.as_ref().unwrap();
                start = (start + end) / 2 + 1;
            }
            count = count - 1;
        }

        return proof;
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {

    let mut proofvec = proof.to_vec();
    proofvec.reverse();
    let mut current = datum.clone();

    let mut number = index;
    let mut indexvector = vec![];
    for i in 0..proof.len() {
        indexvector.push(number % 2);
        number = number / 2;
    }


    for i in 0..proof.len() {
        //if proof says left, we take right
        if indexvector[i] % 2 == 0 {
            current = hash_children(&current, &proofvec[i])
        }
        //if proof says right, we take left
        else{
            current = hash_children(&proofvec[i], &current)
        }
    }

    return current == root.clone(); 

}

#[cfg(test)]
mod tests {
    use crate::crypto::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}
