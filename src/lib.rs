pub mod environment;
pub mod errors;
pub mod interpreter;
pub mod parser;
pub mod resolver;
pub mod scanner;
pub mod syntax;
pub mod tokens;
pub mod types;

use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    process::exit,
};

use interpreter::*;
use parser::*;
use scanner::*;
use tokens::*;

use anyhow::Result;

pub struct Lox {
    interpreter: Interpreter,
    had_runtime_error: bool,
}

impl Lox {
    fn new() -> Self {
        Self {
            interpreter: Interpreter::new(io::stdout()),
            had_runtime_error: false,
        }
    }

    fn run_file<P: AsRef<Path>>(&mut self, file: P) -> Result<()> {
        let script = fs::read_to_string(file)?;
        self.run(script)?;
        // if self.reporter.borrow().had_error {
        //     exit(65);
        // }
        // if self.had_runtime_error {
        //     exit(70);
        // }
        Ok(())
    }

    fn run_repl(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        loop {
            let out = io::stdout();
            let mut output = out.lock();
            write!(output, "> ")?;
            output.flush()?;

            let mut input = String::new();
            reader.read_line(&mut input)?;
            let res = self.run(input);
            // if let Err(e) = todo!() {
            //     eprintln!("Runtime error:\n{}", e);
            // }
        }
    }

    fn run(&mut self, source: String) -> Result<()> {
        let scanner = Scanner::new(source);
        let tokens: Vec<Token> = scanner.collect();
        let mut parser = Parser::new(tokens);
        let program = parser.parse();
        let mut program = program.unwrap();
        // eprintln!("{:?}", program);
        let result = self.interpreter.interpret(&mut program);
        result?;

        Ok(())
    }
}

// pub(crate) struct Reporter {
//     had_error: bool,
// }

// impl Reporter {
//     fn report<S: Into<String>>(&mut self, line: u32, where_: String, message: S) {
//         eprintln!("[line {}] Error{}: {}", line, where_, message.into());
//         self.had_error = true;
//     }

//     fn error<S: Into<String>>(&mut self, line: u32, message: S) {
//         self.report(line, "".to_owned(), message);
//     }

//     fn with_token<S: Into<String>>(&mut self, token: Token, message: S) {
//         let where_ = if token.type_ == TokenType::Eof {
//             " at end".to_owned()
//         } else {
//             " at '".to_owned() + &token.lexeme + "'"
//         };
//         self.report(token.line, where_, message);
//     }
// }
