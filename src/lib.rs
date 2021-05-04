pub mod ast;
pub mod environment;
pub mod errors;
pub mod interpreter;
pub mod parser;
pub mod resolver;
pub mod tokenizer;
pub mod tokens;
pub mod types;

use std::{fs, path::Path};

use interpreter::*;
use parser::*;
use resolver::Resolver;
use tokenizer::*;
use tokens::*;

use anyhow::Result;

pub struct Lox {
    interpreter: Interpreter<'static>,
}

impl Lox {
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(std::io::stdout()),
        }
    }

    pub fn run_file<P: AsRef<Path>>(&mut self, file: P) -> Result<()> {
        let script = fs::read_to_string(file)?;
        self.run(script)?;
        Ok(())
    }

    pub fn run_repl(&mut self) -> Result<()> {
        let mut rl = rustyline::Editor::<()>::new();
        let mut out = std::io::stdout();
        loop {
            // FIXME: Workaround until rustyline supports mingw
            let rl_prompt =
                if cfg!(all(target_family = "windows", target_env = "gnu")) {
                    use std::io::Write;

                    write!(out, "> ")?;
                    out.flush()?;
                    ""
                } else {
                    "> "
                };

            match rl.readline(rl_prompt) {
                Ok(input) => {
                    rl.add_history_entry(&input);
                    let res = self.run(input);
                    if let Err(e) = res {
                        eprintln!("Runtime error:\n{}", e);
                    }
                }
                Err(rustyline::error::ReadlineError::Eof)
                | Err(rustyline::error::ReadlineError::Interrupted) => {
                    return Ok(())
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn run(&mut self, source: String) -> Result<()> {
        let tokenizer = Tokenizer::new(&source);
        let tokens: Vec<Token> = tokenizer
            .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
            .collect::<std::result::Result<_, errors::TokenizerError>>()?;

        let mut parser = Parser::new(tokens);
        let mut program = parser.parse()?;

        let mut resolver = Resolver::new(&mut self.interpreter.locals);
        resolver.resolve(&mut program)?;

        self.interpreter.interpret(&mut program)?;

        Ok(())
    }
}

pub fn run_tests(dir: impl AsRef<Path>) -> Result<()> {
    let mut any_errors = false;
    for file in fs::read_dir(dir)? {
        let file = file?;
        if file.file_type()?.is_file() {
            eprint!("{}: ", file.path().display());
            let res = run_test(file.path());
            if res.is_err() {
                any_errors = true;
            }
        }
    }
    if !any_errors {
        Ok(())
    } else {
        Err(anyhow::anyhow!("tests failed"))
    }
}

pub fn run_test(file: impl AsRef<Path>) -> Result<()> {
    let source = fs::read_to_string(file)?;
    let mut tokens: Vec<_> = Tokenizer::new(&source)
        .collect::<std::result::Result<_, errors::TokenizerError>>()?;

    // Extract expected output
    let expected: String = tokens
        .iter()
        .filter_map(|t| {
            if t.type_ != TokenType::Comment {
                return None;
            }
            t.lexeme.trim().strip_prefix("// expect: ")
        })
        .zip(std::iter::repeat("\n"))
        .flat_map(|(a, b)| std::array::IntoIter::new([a, b]))
        .collect();

    // Remove comments and whitespaces
    tokens.retain(|x| !x.can_skip());

    let mut parser = Parser::new(tokens);
    let mut program = parser.parse()?;

    let mut output = vec![];
    let mut interpreter = Interpreter::new(&mut output);

    let mut resolver = Resolver::new(&mut interpreter.locals);
    resolver.resolve(&mut program)?;

    interpreter.interpret(&mut program)?;

    drop(interpreter);
    let output = String::from_utf8(output)?;
    if output == expected {
        eprintln!("passed");
        Ok(())
    } else {
        eprintln!("failed");
        eprintln!("    expected {:?}, got {:?}", expected, output);
        Err(anyhow::anyhow!("Test failed"))
    }
}
