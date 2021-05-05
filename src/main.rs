use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

use lox::{run_test, run_tests, Lox};

#[derive(StructOpt)]
struct Opt {
    #[structopt(short = "t", long = "test")]
    test: bool,
    input: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut lox = Lox::new();
    match opt.input {
        Some(path) if opt.test && path.is_file() => run_test(path)?,
        Some(path) if opt.test && path.is_dir() => run_tests(path)?,
        None if opt.test => run_tests("./tests")?,
        Some(file) => lox.run_file(file)?,
        None => lox.run_repl()?,
    }
    Ok(())
}
