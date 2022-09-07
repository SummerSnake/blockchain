use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::{block::Block, transaction::Transaction, utxo_set::UTXOSet};
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

const KNOEW_NODE_01: &str = "localhost: 3000";
const CMD_LEN: usize = 12;
const VERSION: i32 = 1;
