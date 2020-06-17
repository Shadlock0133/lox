use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

use lox::Lox;

#[derive(StructOpt)]
struct Opt {
    input_file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut lox = Lox::new();
    match opt.input_file {
        Some(file) => lox.run_file(file)?,
        None => lox.run_repl()?,
    }
    Ok(())
}
