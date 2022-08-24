use super::*;
use crate::blockchain::*;
use crate::transction::*;
use clap::{Arg, Command};
use log::info;
use std::process::exit;

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
            .get_matches();

        // 创建区块链
        if let Some(ref matches) = matches.subcommand_matches("create_blockchain") {
            if let Some(address) = matches.get_one::<String>("address") {
                let address = String::from(address);
                Blockchain::create_blockchain(address.clone())?;
                println!("Create blockchain success.");
            }
        }

        // 打印区块链
        if let Some(_) = matches.subcommand_matches("print_chain") {
            let bc = Blockchain::new()?;

            for b in bc.iter() {
                println!("block: {:#?}", b);
            }
        }

        // 获取余额
        if let Some(ref matches) = matches.subcommand_matches("get_balance") {
            if let Some(address) = matches.get_one::<String>("address") {
                let bc = Blockchain::new()?;
                let utxos = bc.find_utxo(&address);

                let mut balance = 0;
                for out in utxos {
                    balance += out.value;
                }

                println!("Balance of '{}': {}\n", address, balance);
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

            let mut bc = Blockchain::new()?;
            let tx = Transction::new_utxo(from, to, amount, &bc)?;
            bc.mine_block(vec![tx])?;
            println!("Send success");
        }

        Ok(())
    }
}
