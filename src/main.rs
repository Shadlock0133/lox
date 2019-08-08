#![warn(clippy::all)]
use std::{
    cell::RefCell,
    error::Error,
    env::args,
    fs,
    io::{self, Write, BufRead},
    path::Path,
    process::exit,
    rc::Rc,
};

mod tokens;
mod syntax;
mod parser;
mod visitor;
mod interpreter;
use tokens::*;
use syntax::*;
use parser::*;
use visitor::*;
use interpreter::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut syntax = Expr::binary(
        Expr::unary(
            Token::new(TokenType::Minus, "-".to_owned(), None, 1),
            Expr::literal(Value::Number(123.0))
        ),
        Token::new(TokenType::Star, "*".to_owned(), None, 1),
        Expr::grouping(
            Expr::literal(Value::Number(45.67))
        ),
    );
    eprintln!("{}", Printer.visit(&mut syntax));

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
        },
    }
    Ok(())
}

struct Lox {
    reporter: Rc<RefCell<Reporter>>,
}

impl Lox {
    fn new() -> Self {
        let reporter = Reporter { had_error: false };
        Self { reporter: Rc::new(RefCell::new(reporter)) }
    }

    fn run_file<P: AsRef<Path>>(&mut self, file: P) -> Result<(), Box<dyn Error>> {
        let script = fs::read_to_string(file)?;
        self.run(script)?;
        if self.reporter.borrow().had_error { exit(65); }
        Ok(())
    }

    fn run_repl(&mut self, ) -> Result<(), Box<dyn Error>> {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        loop {
            let out = io::stdout();
            let mut output = out.lock();
            write!(output, "> ")?;
            output.flush()?;

            let mut input = String::new();
            reader.read_line(&mut input)?;
            self.run(input)?;
            self.reporter.borrow_mut().had_error = false;
        }
    }

    fn run(&mut self, source: String) -> Result<(), Box<dyn Error>> {
        let scanner = Scanner::new(source, Rc::clone(&self.reporter));
        let tokens: Vec<Token> = scanner.collect();
        let mut parser = Parser::new(tokens, Rc::clone(&self.reporter));
        let expr = parser.parse();
        if self.reporter.borrow().had_error { return Ok(()) }
        let mut expr = expr.unwrap();
        let print = Printer.visit(&mut expr);
        let result = Interpreter.visit(&mut expr);
        if result.is_err() { return Ok(()); }
        println!("{} = {:?}", print, result.unwrap());

        Ok(())
    }
}

pub struct Reporter {
    had_error: bool,
}

impl Reporter {
    fn error<S: AsRef<str>>(&mut self, line: u32, message: S) {
        self.report(line, "".to_owned(), message.as_ref());
    }

    fn report<S: AsRef<str>>(&mut self, line: u32, where_: String, message: S) {
        eprintln!("[line {}] Error{}: {}", line, where_, message.as_ref());
        self.had_error = true;
    }

    fn from_token<S: AsRef<str>>(&mut self, token: Token, message: S) {
        let where_ = if token.type_ == TokenType::Eof {
            " at end".to_owned()
        } else {
            " at '".to_owned() + &token.lexeme + "'"
        };
        self.report(token.line, where_, message);
    }
}