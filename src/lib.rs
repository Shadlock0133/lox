// pub mod environment;
pub mod errors;
// pub mod interpreter;
pub mod parser;
// pub mod resolver;
pub mod ast;
pub mod tokenizer;
pub mod tokens;
pub mod types;

use std::{fs, path::Path};

// use interpreter::*;
use parser::*;
use tokenizer::*;
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
        use std::io::Write;

        let mut rl = rustyline::Editor::<()>::new();
        let mut out = std::io::stdout();
        loop {
            // Workaround until rustyline supports mingw
            write!(out, "> ")?;
            out.flush()?;

            match rl.readline("") {
                Ok(input) => {
                    let res = self.run(input);
                    if let Err(e) = res {
                        eprintln!("Runtime error:\n{}", e);
                    }
                }
                Err(rustyline::error::ReadlineError::Eof)
                | Err(rustyline::error::ReadlineError::Interrupted) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn run(&mut self, source: String) -> Result<()> {
        let scanner = Scanner::new(source);
        let tokens: Vec<Token> = scanner
            .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
            .collect::<std::result::Result<_, errors::TokenError>>()?;
        // eprintln!("{:#?}", tokens);
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;
        // eprintln!("{:?}", program);
        // self.interpreter.interpret(&mut program)?;

        Ok(())
    }
}
