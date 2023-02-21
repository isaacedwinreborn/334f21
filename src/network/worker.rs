use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::crypto::hash::{H256, Hashable};
use crate::block::{Block, Header, Content};
use std::sync::{Arc, Mutex};
use crossbeam::channel;
use log::{debug, warn};

use std::thread;
use std::time;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
}


pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
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
            let mut blockchain = self.blockchain.lock().unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(list_of_hashes) => {
                    //debug!("New Block Hashes: {:?}", list_of_hashes);
                    let mut hash_vec: Vec<H256> = vec![];
                    for hash in list_of_hashes{
                        if blockchain.all_blocks_in_longest_chain().contains(&hash){
                            continue;
                        }else {
                            hash_vec.push(hash);
                        }

                    }
                    if hash_vec.len() != 0{
                        peer.write(Message::GetBlocks(hash_vec));
                    }
                    //if the miner generates a new block, broadcast that message
                    //if the hashes are not already in the blockchain
                }
                Message::GetBlocks(list_of_hashes) => {
                    //debug!("Get Blocks: {:?}", list_of_hashes);
                    let mut block_vec: Vec<Block> = vec![];
                    for hash in list_of_hashes{
                        if blockchain.all_blocks_in_longest_chain().contains(&hash){
                            let block = blockchain.get_block(&hash);
                            block_vec.push(block.clone());
                        }
                    }
                    if block_vec.len() != 0{
                        peer.write(Message::Blocks(block_vec));
                    }

                }
                Message::Blocks(blocks) => {
                    //debug!("Blocks: {:?} \n", blocks);
                    //create a list of added hashes vector
                    let mut added_hashes: Vec<H256> = vec![];
                    for each_block in blocks{
                        let curr_time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_millis();
                        let timestamp = each_block.header.timestamp;
                        print!("delay is {:?} ms \n", curr_time - timestamp);
                        //check if the block belongs to the blockchain
                        let block_hash = each_block.hash();
                        if blockchain.all_blocks_in_longest_chain().contains(&block_hash){
                            continue;
                        }else{
                            let difficulty = blockchain.get_difficulty();
                            if block_hash <= difficulty {
                                let parent = each_block.header.parent;
                                if blockchain.all_blocks_in_longest_chain().contains(&parent){
                                    blockchain.insert(&each_block);
                                    added_hashes.push(block_hash);
                                    //orphan block handler
                                    blockchain.update_orphan_buffer_and_blockchain(&each_block);
                                } else{
                                    blockchain.insert_block_to_orphan_buffer(&each_block);
                                    peer.write(Message::GetBlocks(vec![parent]));
                                }
                            }
                        }
                    }
                    if added_hashes.len() != 0{
                        self.server.broadcast(Message::NewBlockHashes(added_hashes));
                    }
                }

            }

        }
    }
}
