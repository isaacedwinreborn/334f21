use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::crypto::hash::{Hashable, H256};
use crate::blockchain::BlockOrigin;
use crate::transaction::SignedTransaction;
use crate::transaction::State;
use crate::address::H160;

use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool: &Arc<Mutex<Mempool>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mempool: Arc::clone(mempool),
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(hashes) => {
                    debug!("NewBlockHashes: {:?}", hashes);
                    let blockchain = self.blockchain.lock().unwrap();
                    let missing_hashes: Vec<_> = hashes.into_iter()
                        .filter(|hash| !blockchain.contains_block(hash))
                        .collect();
                    if !missing_hashes.is_empty() {
                        peer.write(Message::GetBlocks(missing_hashes));
                    }
                }
                Message::GetBlocks(hashes) => {
                    debug!("GetBlocks: {:?}", hashes);
                    let blockchain = self.blockchain.lock().unwrap();
                    let blocks: Vec<_> = hashes.iter()
                        .filter(|hash| blockchain.contains_block(hash))
                        .map(|hash| blockchain.get_block(hash).clone())
                        .collect();
                    if !blocks.is_empty() {
                        peer.write(Message::Blocks(blocks));
                    }
                }
                Message::Blocks(blocks) => {
                    debug!("Blocks: {:?}", blocks);
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    let mut blockchain = self.blockchain.lock().unwrap();
                    let mut relay_hashes = Vec::new();
                    let mut missing_hashes = Vec::new();
                    for block in blocks {
                        // For experiment: record the block delay; don't count redundant or self-mined blocks:
                        blockchain.hash_to_origin.entry(block.hash())
                            .or_insert(BlockOrigin::Received{ delay_ms: now - block.header.timestamp });
                        // Regular processing:
                        if blockchain.contains_block(&block.hash()) {
                            continue;
                        }
                        if !blockchain.pow_validity_check(&block) {
                            warn!("PoW check failed");
                            continue;
                        }
                        if !blockchain.parent_check(&block) {
                            blockchain.add_to_orphan_buffer(&block);
                            missing_hashes.push(block.header.parent);
                            continue;
                        }
                        let tx_vec = block.content.transactions.clone();
                        //DO TRANSACTION CHECKS
                        peer.write(Message::Transactions(tx_vec));

                        blockchain.insert_recursively(&block, &mut relay_hashes);
                    }
                    if !missing_hashes.is_empty() {
                        peer.write(Message::GetBlocks(missing_hashes));
                    }
                    if !relay_hashes.is_empty() {
                        self.server.broadcast(Message::NewBlockHashes(relay_hashes));
                    }
                }
                Message::NewTransactionHashes(hashes) => {
                    //request from the sender the transactions not yet in the mempool
                    debug!("NewTransactionHashes: {:?}", hashes);
                    let mempool = self.mempool.lock().unwrap();
                    let mut missing_hashes = vec![];
                    for hash in hashes {
                        let this_tx = mempool.get_transaction(&hash);
                        print!("{:?}", this_tx);
                        //if the transaction doesn't exist in mempool, add to missing hashes
                        //the transaction always exists in mempool
                        if this_tx.is_none(){
                            missing_hashes.push(hash);
                        }
                    }
                    //missing hashes is always empty, so we never call GetTransactions
                    if !missing_hashes.is_empty() {
                        peer.write(Message::GetTransactions(missing_hashes));
                    }
                }
                Message::GetTransactions(transaction_hashes) => {
                    //send corresponding transactions to sender
                    debug!("GetTransactions: {:?}", transaction_hashes);
                    let mempool = self.mempool.lock().unwrap();
                    let mut tx_list: Vec<SignedTransaction> = vec![];
                    for tx_hash in transaction_hashes {
                        let this_tx = mempool.get_transaction(&tx_hash).unwrap();
                        tx_list.push(this_tx.clone());
                    }
                    if !tx_list.is_empty() {
                        peer.write(Message::Transactions(tx_list));
                    }
                }
                Message::Transactions(transactions) => {
                    debug!("Transactions: {:?}", transactions);
                    let mut blockchain = self.blockchain.lock().unwrap();
                    let mut mempool = self.mempool.lock().unwrap();
                    let tip = blockchain.tip();
                    let mut valid_tx: Vec<H256> = Vec::new();
                    for tx in transactions {
                        if !tx.verify_signature(){
                            continue;
                        }
                        let mut valid = true;
                        //signature check
                        let publickey = tx.pub_key.clone();
                        let transaction_raw = tx.raw.clone();
                        let transaction_raw_input = transaction_raw.TransactionInput;
                        let curr_state = blockchain.get_state(&tip.clone());
                        for each_input in transaction_raw_input{
                            let each_output = curr_state.get(&each_input);
                            //check if input values actually exist
                            if each_output.is_none(){
                                valid = false;
                                break;
                            }
                            let address = each_output.unwrap().recipient;
                            let pka = H160::from_pubkey(&publickey);
                            if address != pka{
                                valid = false;
                                break;
                            }
                            //check that values of inputs are not less than outputs
                            let input_value = each_input.prev_tx;
                            let output_value = each_output.unwrap().value;
                            if input_value < output_value{
                                valid = false;
                                break;
                            }
                        }
                        if valid {
                            valid_tx.push(tx.hash());
                            mempool.insert(tx);
                        }
                    }
                    /* if !missing_hashes.is_empty() {
                        peer.write(Message::GetTransactions(missing_hashes));
                    } */
                    if !valid_tx.is_empty() {
                        self.server.broadcast(Message::NewTransactionHashes(valid_tx));
                    }
                }
            }
        }
    }
}
