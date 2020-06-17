// pub mod environment;
pub mod errors;
// pub mod interpreter;
pub mod parser;
// pub mod resolver;
pub mod scanner;
pub mod syntax;
pub mod tokens;
pub mod types;

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

// use interpreter::*;
use parser::*;
use scanner::*;
use tokens::*;

use anyhow::Result;

pub struct Lox {
    // interpreter: Interpreter,
}

impl Lox {
    pub fn new() -> Self {
        Self {
            // interpreter: Interpreter::new(io::stdout()),
        }
    }

    pub fn run_file<P: AsRef<Path>>(&mut self, file: P) -> Result<()> {
        let script = fs::read_to_string(file)?;
        self.run(script)?;
        Ok(())
    }

    pub fn run_repl(&mut self) -> Result<()> {
        let reader = io::stdin();
        loop {
            let out = io::stdout();
            let mut output = out.lock();
            write!(output, "> ")?;
            output.flush()?;

            let mut input = String::new();
            reader.read_line(&mut input)?;
            if input.trim().is_empty() {
                break Ok(());
            }
            let res = self.run(input);
            if let Err(e) = res {
                eprintln!("Runtime error:\n{}", e);
            }
        }
    }

    fn run(&mut self, source: String) -> Result<()> {
        let scanner = Scanner::new(source);
        let tokens: Vec<Token> = scanner
            .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
            .collect::<std::result::Result<_, errors::TokenError>>()?;
        eprintln!("{:#?}", tokens);
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;
        eprintln!("{:?}", program);
        // let result = self.interpreter.interpret(&mut program);
        // result?;

        Ok(())
    }
}
