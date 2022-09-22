use std::process::exit;

use super::Result;
use crate::{blockchain::*, server::*, transaction::*, utxo_set::*, wallets::*};
use bitcoincash_addr::Address;
use clap::{Arg, Command};
use log::info;

pub struct Cli {}

impl Cli {
    pub fn new() -> Cli {
        Cli {}
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Run app.");

        let matches = Command::new("blockchain")
            .version("0.1.0")
            .author("SummerSnake")
            .about("A simple blockchain for learning.")
            .subcommand(Command::new("print_chain").about("Print all the chain blocks."))
            .subcommand(Command::new("create_wallets").about("Create a wallet."))
            .subcommand(Command::new("list_addresses").about("List all addresses."))
            .subcommand(Command::new("reindex").about("Reindex UTXO."))
            .subcommand(
                Command::new("get_balance")
                    .about("Get balance in the blockchain.")
                    .arg(Arg::new("address").takes_value(true)),
            )
            .subcommand(
                Command::new("create_blockchain")
                    .about("Create blockchain.")
                    .arg(Arg::new("address")),
            )
            .subcommand(
                Command::new("send")
                    .about("Send in the blockchain.")
                    .arg(Arg::new("from"))
                    .arg(Arg::new("to"))
                    .arg(Arg::new("amount")),
            )
            .subcommand(
                Command::new("start_node")
                    .about("Start the node server.")
                    .arg(Arg::new("port").takes_value(true)),
            )
            .subcommand(
                Command::new("start_miner")
                    .about("Start the miner server.")
                    .arg(Arg::new("port"))
                    .arg(Arg::new("address")),
            )
            .get_matches();

        // 创建区块链
        if let Some(ref matches) = matches.subcommand_matches("create_blockchain") {
            if let Some(address) = matches.get_one::<String>("address") {
                let address = String::from(address);
                let bc = Blockchain::create_blockchain(address)?;
                let utxo_set = UTXOSet { blockchain: bc };
                utxo_set.reindex()?;

                println!("Create blockchain success.");
            }
        }

        // 创建钱包
        if let Some(_) = matches.subcommand_matches("create_wallets") {
            let mut wlts = Wallets::new()?;
            let address = wlts.create_wallet();
            wlts.save_all()?;

            println!("Create wallets success, the wallets address: {}", address);
        }

        // 打印区块链
        if let Some(_) = matches.subcommand_matches("print_chain") {
            let bc = Blockchain::new()?;

            for b in bc.iter() {
                println!("block: {:#?}", b);
            }
        }

        // 打印所有钱包地址
        if let Some(_) = matches.subcommand_matches("list_addresses") {
            let wlt = Wallets::new()?;
            let addresses = wlt.get_all_addresses();

            println!("addresses: ");
            for addr in addresses {
                println!("{}", addr);
            }
        }

        // 重新构建 UTXO 集
        if let Some(_) = matches.subcommand_matches("reindex") {
            let bc = Blockchain::new()?;
            let utxo_set = UTXOSet { blockchain: bc };
            utxo_set.reindex()?;

            let count = utxo_set.count_transactions()?;

            println!("Done! There are {} transactions in the UTXO set.", count);
        }

        // 获取余额
        if let Some(ref matches) = matches.subcommand_matches("get_balance") {
            if let Some(address) = matches.get_one::<String>("address") {
                let pub_key_hash = Address::decode(address).unwrap().body;
                let bc = Blockchain::new()?;
                let utxo_set = UTXOSet { blockchain: bc };
                let utxos = utxo_set.find_utxos(&pub_key_hash)?;

                let mut balance = 0;
                for out in utxos.outputs {
                    balance += out.value;
                }

                println!("Balance: {}\n", balance);
            }
        }

        // 发送交易
        if let Some(ref matches) = matches.subcommand_matches("send") {
            let from = if let Some(address) = matches.get_one::<String>("from") {
                address
            } else {
                println!("From not supply!: usage\n{}", matches.args_present());
                exit(1)
            };

            let to = if let Some(address) = matches.get_one::<String>("to") {
                address
            } else {
                println!("To not supply!: usage\n{}", matches.args_present());
                exit(1)
            };

            let amount: i32 = if let Some(amount) = matches.get_one::<String>("amount") {
                amount.parse()?
            } else {
                println!(
                    "Amount in send not supply!: usage\n{}",
                    matches.args_present()
                );
                exit(1)
            };

            let bc = Blockchain::new()?;
            let mut utxo_set = UTXOSet { blockchain: bc };
            let wlts = Wallets::new()?;
            let wlt = wlts.get_wallet(from).unwrap();
            let tx = Transaction::new_utxo(wlt, to, amount, &utxo_set)?;

            if matches.is_present("mine") {
                let cbtx = Transaction::new_coinbase(from.to_string(), String::from("reward!"))?;
                let new_block = utxo_set.blockchain.mine_block(vec![cbtx, tx])?;

                utxo_set.update(&new_block)?;
            } else {
                Server::send_transaction(&tx, utxo_set)?;
            }
            println!("Send success");
        }

        // 开始节点
        if let Some(ref matches) = matches.subcommand_matches("start_node") {
            if let Some(port) = matches.get_one::<String>("port") {
                println!("Start node...");

                let bc = Blockchain::new()?;
                let utxo_set = UTXOSet { blockchain: bc };
                let server = Server::new(port, "", utxo_set)?;
                server.start_server()?;
            }
        }

        // 矿工节点
        if let Some(ref matches) = matches.subcommand_matches("start_miner") {
            let address = if let Some(address) = matches.get_one::<String>("address") {
                address
            } else {
                println!("From not supply!: usage\n{}", matches.args_present());
                exit(1)
            };

            let port = if let Some(port) = matches.get_one::<String>("port") {
                port
            } else {
                println!("From not supply!: usage\n{}", matches.args_present());
                exit(1)
            };

            println!("Start miner node...");
            let bc = Blockchain::new()?;
            let utxo_set = UTXOSet { blockchain: bc };
            let server = Server::new(port, address, utxo_set)?;
            server.start_server()?;
        }

        Ok(())
    }
}
