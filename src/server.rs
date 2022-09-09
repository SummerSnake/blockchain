use std::{
    collections::{HashMap, HashSet},
    io::prelude::Write,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::Result;
use crate::{block::Block, transaction::Transaction, utxo_set::UTXOSet};
use bincode::{deserialize, serialize};
use log::info;
use serde::{Deserialize, Serialize};

// 消息
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
struct VersionMsg {
    addr_from: String,
    version: i32,
    best_height: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct TxMsg {
    addr_from: String,
    transaction: Transaction,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct GetDataMsg {
    addr_from: String,
    kind: String,
    id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct GetBlockMsg {
    addr_from: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct InvMsg {
    addr_from: String,
    kind: String,
    items: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BlockMsg {
    addr_from: String,
    block: Block,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
enum Message {
    Addr(Vec<String>),
    Version(VersionMsg),
    Tx(TxMsg),
    GetData(GetDataMsg),
    GetBlock(GetBlockMsg),
    Inv(InvMsg),
    Block(BlockMsg),
}

// 服务
struct ServerInner {
    known_nodes: HashSet<String>,
    utxo: UTXOSet,
    blocks_in_transit: Vec<String>,
    mempool: HashMap<String, Transaction>,
}
pub struct Server {
    node_address: String,
    mining_address: String,
    inner: Arc<Mutex<ServerInner>>,
}

const KNOWN_NODE_01: &str = "localhost: 3000";
const CMD_LEN: usize = 12;
const VERSION: i32 = 1;

impl Server {
    pub fn new(port: &str, miner_address: &str, utxo: UTXOSet) -> Result<Server> {
        let mut node_set = HashSet::new();
        node_set.insert(String::from(KNOWN_NODE_01));

        Ok(Server {
            node_address: String::from("localhost") + port,
            mining_address: miner_address.to_string(),
            inner: Arc::new(Mutex::new(ServerInner {
                known_nodes: node_set,
                utxo,
                blocks_in_transit: Vec::new(),
                mempool: HashMap::new(),
            })),
        })
    }

    pub fn start_server(&self) -> Result<()> {
        let server_01 = Server {
            node_address: self.node_address.clone(),
            mining_address: self.mining_address.clone(),
            inner: Arc::clone(&self.inner),
        };

        info!(
            "Start server at {}, mining address: {}",
            &self.node_address, &self.mining_address
        );

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));

            if server_01.get_best_height()? == -1 {
                server_01.request_blocks()
            } else {
                server_01.send_version(KNOWN_NODE_01)
            }
        });

        Ok(())
    }

    fn get_best_height(&self) -> Result<i32> {
        self.inner.lock().unwrap().utxo.blockchain.get_best_height()
    }

    fn get_known_nodes(&self) -> HashSet<String> {
        self.inner.lock().unwrap().known_nodes.clone()
    }

    fn remove_node(&self, addr: &str) {
        self.inner.lock().unwrap().known_nodes.remove(addr);
    }

    fn send_data(&self, addr: &str, data: &[u8]) -> Result<()> {
        if addr == &self.node_address {
            return Ok(());
        }

        let mut stream = match TcpStream::connect(addr) {
            Ok(s) => s,
            Err(_) => {
                self.remove_node(addr);
                return Ok(());
            }
        };

        stream.write(data)?;
        info!("Data send successfully");
        Ok(())
    }

    fn send_get_blocks(&self, addr: &str) -> Result<()> {
        info!("Send get blocks message to: {}", addr);

        let data = GetBlockMsg {
            addr_from: self.node_address.clone(),
        };
        let data = serialize(&(cmd_to_bytes("get_blocks"), data))?;
        self.send_data(addr, &data)
    }

    fn request_blocks(&self) -> Result<()> {
        for node in self.get_known_nodes() {
            self.send_get_blocks(&node)?
        }

        Ok(())
    }

    fn send_version(&self, addr: &str) -> Result<()> {
        info!("Send version info to: {}", addr);

        let data = VersionMsg {
            addr_from: self.node_address.clone(),
            best_height: self.get_best_height()?,
            version: VERSION,
        };
        let data = serialize(&(cmd_to_bytes("version"), data))?;
        self.send_data(addr, &data)
    }
}

fn cmd_to_bytes(cmd: &str) -> [u8; CMD_LEN] {
    let mut data = [0; CMD_LEN];

    for (i, d) in cmd.as_bytes().iter().enumerate() {
        data[i] = *d;
    }

    data
}
