use clap::Parser;
use tensor_vm::{TvmdCli, app::execute_tvmd_command};

fn main() {
    let command = TvmdCli::parse().command;
    match execute_tvmd_command(&command) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
