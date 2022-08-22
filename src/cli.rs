use super::*;
use crate::blockchain::*;
use clap::{App, Arg};
use log::info;

pub struct Cli {
    bc: Blockchain,
}

impl Cli {
    pub fn new() -> Result<Cli> {
        Ok(Cli {
            bc: Blockchain::new()?,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Run app.");

        let matches = App::new("blockchain")
            .version("0.1.0")
            .author("SummerSnake")
            .about("A simple blockchain for learning.")
            .subcommand(App::new("print_chain").about("Print all the chain blocks."))
            .subcommand(
                App::new("add_block")
                    .about("Add a block to the blockchain")
                    .arg(Arg::from_usage("<data> 'the blockchain data'")),
            )
            .get_matches();

        if let Some(ref matches) = matches.subcommand_matches("add_block") {
            if let Some(c) = matches.value_of("data") {
                self.add_block(String::from(c))?;
            } else {
                println!("Not printing testing lists...");
            }
        }

        if let Some(_) = matches.subcommand_matches("print_chain") {
            self.print_chain();
        }

        Ok(())
    }

    fn add_block(&mut self, data: String) -> Result<()> {
        self.bc.add_block(data)
    }

    fn print_chain(&mut self) {
        for b in &mut self.bc {
            println!("block: {:#?}", b);
        }
    }
}
