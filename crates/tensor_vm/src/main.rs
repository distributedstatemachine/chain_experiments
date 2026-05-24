use clap::Parser;
use tensor_vm::{TvmdCli, app};

fn main() {
    match app::run(TvmdCli::parse()) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
