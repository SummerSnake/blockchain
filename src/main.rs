mod block;
mod blockchain;
mod cli;
mod transaction;
mod utxo_set;
mod wallets;

pub type Result<T> = std::result::Result<T, failure::Error>;

use cli::Cli;
use env_logger::Env;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warning...")).init();

    let mut cli = Cli::new();
    if let Err(e) = cli.run() {
        println!("Error: {}", e);
    };
}
