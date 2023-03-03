use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use crate::crypto::hash::{H256, Hashable};

use crate::network::server::Handle as ServerHandle;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use crate::mempool::Mempool;
use crate::network::message::Message;
use crate::transaction::{SignedTransaction, Transaction, TransactionInput, TransactionOutput, State};
use crate::blockchain::{Blockchain};
use crate::address::H160;
use rand::prelude::*;
use crate::crypto::key_pair;

pub struct TransactionGenerator {
    server: ServerHandle,
    mempool: Arc<Mutex<Mempool>>,
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub controlled_keypair: Ed25519KeyPair,
}

impl TransactionGenerator {
    pub fn new(
        server: &ServerHandle,
        mempool: &Arc<Mutex<Mempool>>,
        blockchain: &Arc<Mutex<Blockchain>>,
        controlled_keypair: Ed25519KeyPair
    ) -> TransactionGenerator {
        TransactionGenerator {
            server: server.clone(),
            mempool: Arc::clone(mempool),
            blockchain: Arc::clone(blockchain),
            controlled_keypair,
        }
    }

    pub fn start(self) {
        thread::spawn(move || {
            self.generation_loop();
            log::warn!("Transaction Generator exited");
        });
    }

    /// Generate random transactions and send them to the server
    fn generation_loop(&self) {
        const INTERVAL_MILLISECONDS: u64 = 3000; // how quickly to generate transactions

        loop {
            // sleep for some time:
            let interval = time::Duration::from_millis(INTERVAL_MILLISECONDS);
            thread::sleep(interval);

            let mut mempool = self.mempool.lock().unwrap();

            let keypair = &self.controlled_keypair;

            //read the state and check which UTXO to use
            let blockchain = self.blockchain.lock().unwrap();
            let tip = blockchain.tip();
            let state_to_read = blockchain.get_state(&tip);
            let mut state_to_use = state_to_read.clone();
            let mut input_vec: Vec<TransactionInput> = vec![];
            let mut output_vec: Vec<TransactionOutput> = vec![];
            for  (tx_input, tx_output) in state_to_read.iter() {
                if tx_output.recipient != H160::from_pubkey(&keypair.public_key().as_ref()){
                    state_to_use.remove(tx_input);
                    continue;
                } else{
                    input_vec.push(tx_input.clone());
                    output_vec.push(tx_output.clone());
                }
            }
              
            // 1. generate some random transactions:
            let trans = Transaction{
                TransactionInput: input_vec,
                TransactionOutput: output_vec,
            };
            let signed_trans = SignedTransaction::from_raw(trans, &key_pair::random()).clone();

            // 2. add these transactions to the mempool:
            let mut t_hash: Vec<H256> = vec![];
            mempool.insert(signed_trans.clone());
            let transaction_hash = &signed_trans.hash();
            t_hash.push(transaction_hash.clone());
            // 3. broadcast them using `self.server.broadcast(Message::NewTransactionHashes(...))`:
            self.server.broadcast(Message::NewTransactionHashes(t_hash));
        }
    }
}