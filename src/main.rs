use std::{path::PathBuf, str::FromStr};

use anyhow::Result;
use structopt::StructOpt;

use lox::{CLox, JLox};

enum Backend {
    JLox,
    CLox,
}

impl FromStr for Backend {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jlox" => Ok(Self::JLox),
            "clox" => Ok(Self::CLox),
            _ => Err("Unsupported backend\nAvailble backends: jlox, clox"),
        }
    }
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    test: bool,
    #[structopt(short, long)]
    debug: bool,
    #[structopt(short, long)]
    backend: Backend,
    input: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    match opt.backend {
        Backend::JLox => match opt.input {
            Some(path) if opt.test && path.is_file() => JLox::run_test(path)?,
            Some(path) if opt.test && path.is_dir() => JLox::run_tests(path)?,
            None if opt.test => JLox::run_tests("./tests")?,
            Some(file) => JLox::new().run_file(file)?,
            None => JLox::new().run_repl()?,
        },
        Backend::CLox => match opt.input {
            Some(path) => CLox::new(opt.debug).run_file(path)?,
            None => CLox::new(opt.debug).run_repl()?,
        },
    }
    Ok(())
}
