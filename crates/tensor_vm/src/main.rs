use clap::Parser;
use tensor_vm::TvmdCli;

fn main() {
    match TvmdCli::parse().execute() {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
