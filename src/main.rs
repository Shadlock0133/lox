#![warn(clippy::all)]
use std::{
    cell::RefCell,
    env::args,
    error::Error,
    fs,
    io::{self, BufRead, Write},
    path::Path,
    process::exit,
    rc::Rc,
};

mod environment;
mod interpreter;
mod parser;
mod scanner;
mod syntax;
mod tokens;
mod visitor;

use interpreter::*;
use parser::*;
use scanner::*;
use tokens::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = args();
    let input_file = args.nth(1);
    let has_other = args.next().is_some();

    let mut lox = Lox::new();
    match (input_file, has_other) {
        (Some(file), false) => lox.run_file(file)?,
        (None, _) => lox.run_repl()?,
        _ => {
            eprintln!(concat!("Usage: ", env!("CARGO_PKG_NAME"), " [script]"));
            exit(64);
        }
    }
    Ok(())
}

struct Lox {
    interpreter: Interpreter,
    reporter: Rc<RefCell<Reporter>>,
    had_runtime_error: bool,
}

impl Lox {
    fn new() -> Self {
        let reporter = Reporter { had_error: false };
        Self {
            interpreter: Interpreter::new(io::stdout()),
            reporter: Rc::new(RefCell::new(reporter)),
            had_runtime_error: false,
        }
    }

    fn run_file<P: AsRef<Path>>(&mut self, file: P) -> Result<(), Box<dyn Error>> {
        let script = fs::read_to_string(file)?;
        self.run(script)?;
        if self.reporter.borrow().had_error {
            exit(65);
        }
        if self.had_runtime_error {
            exit(70);
        }
        Ok(())
    }

    fn run_repl(&mut self) -> Result<(), Box<dyn Error>> {
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
            self.reporter.borrow_mut().had_error = false;
            if self.had_runtime_error {
                self.had_runtime_error = false;
                eprintln!("Runtime error:\n{}", res.unwrap_err());
            }
        }
    }

    fn run(&mut self, source: String) -> Result<(), Box<dyn Error>> {
        let scanner = Scanner::new(source, Rc::clone(&self.reporter));
        let tokens: Vec<Token> = scanner.collect();
        let mut parser = Parser::new(tokens, Rc::clone(&self.reporter));
        let program = parser.parse();
        if self.reporter.borrow().had_error {
            return Ok(());
        }
        let mut program = program.unwrap();
        // eprintln!("{:?}", program);
        let result = self.interpreter.interpret(&mut program);
        if result.is_err() {
            self.had_runtime_error = true;
        }
        result?;

        Ok(())
    }
}

pub struct Reporter {
    had_error: bool,
}

impl Reporter {
    fn error<S: Into<String>>(&mut self, line: u32, message: S) {
        self.report(line, "".to_owned(), message);
    }

    fn report<S: Into<String>>(&mut self, line: u32, where_: String, message: S) {
        eprintln!("[line {}] Error{}: {}", line, where_, message.into());
        self.had_error = true;
    }

    fn with_token<S: Into<String>>(&mut self, token: Token, message: S) {
        let where_ = if token.type_ == TokenType::Eof {
            " at end".to_owned()
        } else {
            " at '".to_owned() + &token.lexeme + "'"
        };
        self.report(token.line, where_, message);
    }
}
