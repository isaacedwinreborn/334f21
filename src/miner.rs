use crate::network::server::Handle as ServerHandle;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::transaction::SignedTransaction as Transaction;
use crate::crypto::merkle::MerkleTree;
use crate::block::{Block, Header, Content};
use crate::crypto::hash::Hashable;
use crate::network::message::Message;
use crate::blockchain::BlockOrigin;

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    // For experiments:
    total_blocks_mined: u64,
    start_time: Option<SystemTime>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool: &Arc<Mutex<Mempool>>,
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mempool: Arc::clone(mempool),
        total_blocks_mined: 0,
        start_time: None,
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;

                // print mining stats if the miner started:
                if let Some(start_time) = self.start_time {
                    let seconds_spent = SystemTime::now().duration_since(start_time).unwrap().as_secs_f64();
                    let mining_rate = (self.total_blocks_mined as f64) / seconds_spent;
                    info!("Mined {} blocks in {} seconds, rate is {} blocks/second",
                        self.total_blocks_mined, seconds_spent, mining_rate);
                    let blockchain = self.blockchain.lock().unwrap();
                    info!("Blockchain has {} blocks in total", blockchain.block_count());
                    let longest_chain = blockchain.all_blocks_in_longest_chain();
                    info!("Longest chain {:?} has {} blocks", longest_chain, longest_chain.len());
                    info!("Average block size is {} bytes", blockchain.average_block_size());
                    info!("Delays in ms for each block (raw data): {:?}", blockchain.block_delays_ms());
                }
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);

                // set the miner start time:
                if self.start_time == None {
                    self.start_time = Some(SystemTime::now());
                }
            }
        }
    }

    fn miner_loop(&mut self) {
        // main mining loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }


            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }

                let mut blockchain = self.blockchain.lock().unwrap();
                let mut mempool = self.mempool.lock().unwrap();

                let parent = blockchain.tip();
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                let difficulty = blockchain.get_block(&parent).header.difficulty;
                let mut transactions: Vec<Transaction> = vec![Default::default()];
                let merkle_root = MerkleTree::new(&transactions).root();
                let nonce = rand::random();
        
                let header = Header {
                    parent,
                    nonce,
                    difficulty,
                    timestamp,
                    merkle_root, 
                };
                if transactions.len() < 20 {
                    while transactions.len() < 20{
                        let trans = mempool.pop();
                        if !trans.is_none(){
                            transactions.push(trans.unwrap().clone());
                        }
                        else{
                            break;
                        }
                    }
                } 
                let content = Content { transactions };
                let block = Block { header, content };

                if block.hash() <= difficulty {
                    blockchain.insert(&block);
                    self.total_blocks_mined += 1;
                    self.server.broadcast(Message::NewBlockHashes(vec![block.hash()]));
                    blockchain.hash_to_origin.insert(block.hash(), BlockOrigin::Mined);
                }
            }
        }
    }
}
