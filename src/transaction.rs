use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use crate::{crypto::hash::{H256, Hashable}, address::H160};
use std::collections::HashMap;
use std::hash::Hash;


#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash, PartialEq, Eq, Copy)]
pub struct TransactionInput {
    pub txid: u32,
    pub prev_tx: H256,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash, PartialEq, Eq, Copy)]
pub struct TransactionOutput {
    pub recipient: H160,
    pub value: H256,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash)]
pub struct Transaction {
    pub TransactionInput: Vec<TransactionInput>,
    pub TransactionOutput: Vec<TransactionOutput>,
}

pub type State = HashMap<TransactionInput, TransactionOutput>;

/// A signed transaction
#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash)]
pub struct SignedTransaction {
    pub raw: Transaction,  
    pub pub_key: Vec<u8>,
    pub signature: Vec<u8>,
}



impl SignedTransaction {
    /// Create a new transaction from a raw transaction and a key pair
    pub fn from_raw(raw: Transaction, key: &Ed25519KeyPair) -> SignedTransaction {
        let pub_key = key.public_key().as_ref().to_vec();
        let signature = sign(&raw, key).as_ref().to_vec();
        SignedTransaction { raw, pub_key, signature }
    }

    /// Verify the signature of this transaction
    pub fn verify_signature(&self) -> bool {
        let serialized_raw = bincode::serialize(&self.raw).unwrap();
        let public_key = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ED25519, &self.pub_key[..]);
        public_key.verify(&serialized_raw, self.signature.as_ref()).is_ok()
    }
}

impl Hashable for Transaction {
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    key.sign(bincode::serialize(&t).unwrap().as_ref())
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key.as_ref())
        .verify(bincode::serialize(&t).unwrap().as_ref(), signature.as_ref())
        .is_ok()
}


#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    pub fn generate_random_transaction() -> Transaction {
        let private_key = [100u8; 32];
        let controlled_keypair = Ed25519KeyPair::from_seed_unchecked(&private_key).unwrap();
        let trans1 = TransactionInput{
            txid: 1,
            prev_tx: H256::from([50u8; 32]),
        };
        let output1 = TransactionOutput{
            recipient: H160::from_pubkey(&controlled_keypair.public_key().as_ref()),
            value: H256::from([10u8; 32]),
        };
        let trans = Transaction{
            TransactionInput: vec![trans1],
            TransactionOutput: vec![output1],
        };
        return trans;
    }

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
