use std::{
    collections::{HashMap, HashSet},
    io::prelude::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::Result;
use crate::{block::Block, transaction::Transaction, utxo_set::UTXOSet};
use bincode::{deserialize, serialize};
use failure::format_err;
use log::{debug, info};
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
            node_address: String::from("localhost:") + port,
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
            "Start server at {}, mining address: {}.",
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

        let listener = TcpListener::bind(&self.node_address).unwrap();
        info!("Server listen...");

        for stream in listener.incoming() {
            let stream = stream?;
            let server_01 = Server {
                node_address: self.node_address.clone(),
                mining_address: self.mining_address.clone(),
                inner: Arc::clone(&self.inner),
            };

            thread::spawn(move || server_01.handle_connection(stream));
        }

        Ok(())
    }

    pub fn send_transaction(tx: &Transaction, utxoset: UTXOSet) -> Result<()> {
        let server = Server::new("7000", "", utxoset)?;
        server.send_tx(KNOWN_NODE_01, tx)?;

        Ok(())
    }

    fn get_best_height(&self) -> Result<i32> {
        self.inner.lock().unwrap().utxo.blockchain.get_best_height()
    }

    fn get_known_nodes(&self) -> HashSet<String> {
        self.inner.lock().unwrap().known_nodes.clone()
    }

    fn node_is_known(&self, addr: &str) -> bool {
        self.inner.lock().unwrap().known_nodes.get(addr).is_some()
    }

    fn add_nodes(&self, addr: &str) {
        self.inner
            .lock()
            .unwrap()
            .known_nodes
            .insert(String::from(addr));
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
        info!("Data send successfully.");

        Ok(())
    }

    pub fn send_tx(&self, addr: &str, tx: &Transaction) -> Result<()> {
        info!("Send tx to: {} txid: {}.", addr, &tx.id);

        let data = TxMsg {
            addr_from: self.node_address.clone(),
            transaction: tx.clone(),
        };
        let data = serialize(&(cmd_to_bytes("tx"), data))?;
        self.send_data(addr, &data)
    }

    fn send_version(&self, addr: &str) -> Result<()> {
        info!("Send version info to: {}.", addr);

        let data = VersionMsg {
            addr_from: self.node_address.clone(),
            best_height: self.get_best_height()?,
            version: VERSION,
        };
        let data = serialize(&(cmd_to_bytes("version"), data))?;
        self.send_data(addr, &data)
    }

    fn send_inv(&self, addr: &str, kind: &str, items: Vec<String>) -> Result<()> {
        info!(
            "Send inv message to: {} kind: {} data: {:?}",
            addr, kind, items
        );

        let data = InvMsg {
            addr_from: self.node_address.clone(),
            kind: kind.to_string(),
            items,
        };
        let data = serialize(&(cmd_to_bytes("inv"), data))?;

        self.send_data(addr, &data)
    }

    fn send_get_blocks(&self, addr: &str) -> Result<()> {
        info!("Send get blocks message to: {}.", addr);

        let data = GetBlockMsg {
            addr_from: self.node_address.clone(),
        };
        let data = serialize(&(cmd_to_bytes("get_blocks"), data))?;
        self.send_data(addr, &data)
    }

    fn send_get_data(&self, addr: &str, kind: &str, id: &str) -> Result<()> {
        info!(
            "Send get data message to: {} kind: {} id: {}.",
            addr, kind, id
        );

        let data = GetDataMsg {
            addr_from: self.node_address.clone(),
            kind: kind.to_string(),
            id: id.to_string(),
        };
        let data = serialize(&(cmd_to_bytes("get_data"), data))?;
        self.send_data(addr, &data)
    }

    fn send_block(&self, addr: &str, b: &Block) -> Result<()> {
        info!("Send block data to: {} block hash: {}", addr, b.get_hash());

        let data = BlockMsg {
            addr_from: self.node_address.clone(),
            block: b.clone(),
        };
        let data = serialize(&(cmd_to_bytes("block"), data))?;
        self.send_data(addr, &data)
    }

    fn send_addr(&self, addr: &str) -> Result<()> {
        info!("Send address info to: {}.", addr);
        let nodes = self.get_known_nodes();
        let data = serialize(&(cmd_to_bytes("addr"), nodes))?;

        self.send_data(addr, &data)
    }

    fn add_block(&self, block: Block) -> Result<()> {
        self.inner.lock().unwrap().utxo.blockchain.add_block(block)
    }

    fn mine_block(&self, txs: Vec<Transaction>) -> Result<Block> {
        self.inner.lock().unwrap().utxo.blockchain.mine_block(txs)
    }

    fn request_blocks(&self) -> Result<()> {
        for node in self.get_known_nodes() {
            self.send_get_blocks(&node)?
        }

        Ok(())
    }

    fn get_block_hashes(&self) -> Vec<String> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .get_block_hashes()
    }

    fn get_block(&self, block_hash: &str) -> Result<Block> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .get_block(block_hash)
    }

    fn get_in_transit(&self) -> Vec<String> {
        self.inner.lock().unwrap().blocks_in_transit.clone()
    }

    fn get_mempool_tx(&self, addr: &str) -> Option<Transaction> {
        match self.inner.lock().unwrap().mempool.get(addr) {
            Some(tx) => Some(tx.clone()),
            None => None,
        }
    }

    fn verify_tx(&self, tx: &Transaction) -> Result<bool> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .verify_transaction(tx)
    }

    fn get_mempool(&self) -> HashMap<String, Transaction> {
        self.inner.lock().unwrap().mempool.clone()
    }

    fn insert_mempool(&self, tx: Transaction) {
        self.inner.lock().unwrap().mempool.insert(tx.id.clone(), tx);
    }

    fn clear_mempool(&self) {
        self.inner.lock().unwrap().mempool.clear()
    }

    fn replace_in_transit(&self, hashes: Vec<String>) {
        let bit = &mut self.inner.lock().unwrap().blocks_in_transit;
        bit.clone_from(&hashes);
    }

    fn utxo_reindex(&self) -> Result<()> {
        self.inner.lock().unwrap().utxo.reindex()
    }

    fn handle_addr(&self, msg: Vec<String>) -> Result<()> {
        info!("Receive address msg: {:#?}.", msg);

        for node in msg {
            self.add_nodes(&node);
        }

        Ok(())
    }

    fn handle_block(&self, msg: BlockMsg) -> Result<()> {
        info!(
            "Receive block msg: {}, {}.",
            msg.addr_from,
            msg.block.get_hash()
        );
        self.add_block(msg.block)?;

        let mut in_transit = self.get_in_transit();
        if in_transit.len() > 0 {
            let block_hash = &in_transit[0];
            self.send_get_data(&msg.addr_from, "block", block_hash)?;
            in_transit.remove(0);
            self.replace_in_transit(in_transit);
        } else {
            self.utxo_reindex()?;
        }

        Ok(())
    }

    fn handle_inv(&self, msg: InvMsg) -> Result<()> {
        info!("Recieve inv msg: {:#?}.", msg);

        if msg.kind == "block" {
            let block_hash = &msg.items[0];
            self.send_get_data(&msg.addr_from, "block", block_hash)?;

            let mut new_in_transit = Vec::new();
            for b in &msg.items {
                if b != block_hash {
                    new_in_transit.push(b.clone());
                }
            }
        }
        if msg.kind == "tx" {
            let txid = &msg.items[0];

            match self.get_mempool_tx(txid) {
                Some(tx) => {
                    if tx.id.is_empty() {
                        self.send_get_data(&msg.addr_from, "tx", txid)?
                    }
                }
                None => self.send_get_data(&msg.addr_from, "tx", txid)?,
            }
        }

        Ok(())
    }

    fn handle_get_blocks(&self, msg: GetBlockMsg) -> Result<()> {
        info!("Receive get blocks msg: {:#?}.", msg);

        let block_hashes = self.get_block_hashes();
        self.send_inv(&msg.addr_from, "block", block_hashes)?;

        Ok(())
    }

    fn handle_get_data(&self, msg: GetDataMsg) -> Result<()> {
        info!("Receive get data msg: {:#?}.", msg);

        if msg.kind == "block" {
            let block = self.get_block(&msg.id)?;
            self.send_block(&msg.addr_from, &block)?;
        }
        if msg.kind == "tx" {
            let tx = self.get_mempool_tx(&msg.id).unwrap();
            self.send_tx(&msg.addr_from, &tx)?;
        }

        Ok(())
    }

    fn handle_tx(&self, msg: TxMsg) -> Result<()> {
        info!("Receive tx msg: {} {}.", msg.addr_from, &msg.transaction.id);

        self.insert_mempool(msg.transaction.clone());

        let known_nodes = self.get_known_nodes();
        if self.node_address == KNOWN_NODE_01 {
            for node in known_nodes {
                if node != self.node_address && node != msg.addr_from {
                    self.send_inv(&node, "tx", vec![msg.transaction.id.clone()])?;
                }
            }
        } else {
            let mut mempool = self.get_mempool();

            debug!("Current mempool: {:#?}", &mempool);
            if mempool.len() >= 1 && !self.mining_address.is_empty() {
                loop {
                    let mut txs = Vec::new();

                    for (_, tx) in &mempool {
                        if self.verify_tx(tx)? {
                            txs.push(tx.clone());
                        }
                    }

                    if txs.is_empty() {
                        return Ok(());
                    }

                    let cbtx =
                        Transaction::new_coinbase(self.mining_address.clone(), String::new())?;
                    txs.push(cbtx);

                    for tx in &txs {
                        mempool.remove(&tx.id);
                    }

                    let new_block = self.mine_block(txs)?;
                    self.utxo_reindex()?;

                    for node in self.get_known_nodes() {
                        if node != self.node_address {
                            self.send_inv(&node, "block", vec![new_block.get_hash()])?;
                        }
                    }

                    if mempool.len() == 0 {
                        break;
                    }
                }

                self.clear_mempool();
            }
        }

        Ok(())
    }

    fn handle_version(&self, msg: VersionMsg) -> Result<()> {
        info!("Receive version msg: {:#?}", msg);

        let my_best_height = self.get_best_height()?;
        if my_best_height < msg.best_height {
            self.send_get_blocks(&msg.addr_from)?;
        }
        if my_best_height > msg.best_height {
            self.send_version(&msg.addr_from)?;
        }

        self.send_addr(&msg.addr_from)?;

        if !self.node_is_known(&msg.addr_from) {
            self.add_nodes(&msg.addr_from);
        }

        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        let mut buffer = Vec::new();
        let count = stream.read_to_end(&mut buffer)?;
        info!("Accept request: length {}", count);

        let cmd = bytes_to_cmd(&buffer)?;
        match cmd {
            Message::Addr(data) => self.handle_addr(data)?,
            Message::Block(data) => self.handle_block(data)?,
            Message::Inv(data) => self.handle_inv(data)?,
            Message::GetBlock(data) => self.handle_get_blocks(data)?,
            Message::GetData(data) => self.handle_get_data(data)?,
            Message::Tx(data) => self.handle_tx(data)?,
            Message::Version(data) => self.handle_version(data)?,
        }

        Ok(())
    }
}

fn cmd_to_bytes(cmd: &str) -> [u8; CMD_LEN] {
    let mut data = [0; CMD_LEN];

    for (i, d) in cmd.as_bytes().iter().enumerate() {
        data[i] = *d;
    }

    data
}

fn bytes_to_cmd(bytes: &[u8]) -> Result<Message> {
    let mut cmd = Vec::new();
    let cmd_bytes = &bytes[..CMD_LEN];
    let data = &bytes[CMD_LEN..];

    for b in cmd_bytes {
        if 0 as u8 != *b {
            cmd.push(*b);
        }
    }

    info!("cmd: {}", String::from_utf8(cmd.clone())?);

    if cmd == "addr".as_bytes() {
        let data = deserialize(data)?;
        Ok(Message::Addr(data))
    } else if cmd == "block".as_bytes() {
        let data = deserialize(data)?;
        Ok(Message::Block(data))
    } else if cmd == "inv".as_bytes() {
        let data = deserialize(data)?;
        Ok(Message::Inv(data))
    } else if cmd == "get_blocks".as_bytes() {
        let data = deserialize(data)?;
        Ok(Message::GetBlock(data))
    } else if cmd == "get_data".as_bytes() {
        let data = deserialize(data)?;
        Ok(Message::GetData(data))
    } else if cmd == "tx".as_bytes() {
        let data = deserialize(data)?;
        Ok(Message::Tx(data))
    } else if cmd == "version".as_bytes() {
        let data = deserialize(data)?;
        Ok(Message::Version(data))
    } else {
        Err(format_err!("Unknown command in the server."))
    }
}
