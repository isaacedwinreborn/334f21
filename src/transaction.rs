use serde::{Serialize,Deserialize};
use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Transaction {
    input: u32,
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let encoded: Vec<u8> = bincode::serialize(&t).unwrap();
    let sig = key.sign(encoded.as_ref());
    return sig;
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    let encoded: Vec<u8> = bincode::serialize(&t).unwrap();
    let peer_public_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key);
    let result = peer_public_key.verify(encoded.as_ref(), signature.as_ref());
    return result.is_ok();
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    pub fn generate_random_transaction() -> Transaction {
        let t = rand::random::<u32>();
        let t = Transaction{
            input: t
        };
        return t;
    }

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
