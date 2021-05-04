use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

use lox::{run_test, run_tests, Lox};

#[derive(StructOpt)]
struct Opt {
    #[structopt(long = "test")]
    test: bool,
    input_file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut lox = Lox::new();
    match opt.input_file {
        Some(file) if opt.test => run_test(file)?,
        None if opt.test => run_tests("tests")?,
        Some(file) => lox.run_file(file)?,
        None => lox.run_repl()?,
    }
    Ok(())
}
