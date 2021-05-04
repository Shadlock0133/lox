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

const SKIP: &[&str] = &["benchmark", "limit"];

fn run_tests_rec(
    prefix: impl AsRef<Path>,
    dir: impl AsRef<Path>,
    passes: &mut usize,
    fails: &mut usize,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_file() {
            eprint!(
                "test {} ... ",
                path.strip_prefix(prefix.as_ref())?.display()
            );
            let res = run_test(path);
            match res {
                Ok(_) => *passes += 1,
                Err(_) => *fails += 1,
            }
        } else if file_type.is_dir() {
            if !SKIP.iter().any(|x| x == &path.file_name().unwrap()) {
                run_tests_rec(prefix.as_ref(), path, passes, fails)?;
            }
        }
    }
    Ok(())
}

pub fn run_tests(dir: impl AsRef<Path>) -> Result<()> {
    let mut passes = 0;
    let mut fails = 0;
    run_tests_rec(dir.as_ref(), dir.as_ref(), &mut passes, &mut fails)?;
    eprintln!("successes: {}, fails: {}", passes, fails);
    if fails == 0 {
        Ok(())
    } else {
        Err(anyhow::anyhow!("tests failed"))
    }
}

fn run(tokens: Vec<Token>) -> Result<String> {
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse()?;

    let mut output = vec![];
    let mut interpreter = Interpreter::new(&mut output);

    let mut resolver = Resolver::new(&mut interpreter.locals);
    resolver.resolve(&mut program)?;

    interpreter.interpret(&mut program)?;

    drop(interpreter);
    let output = String::from_utf8(output)?;
    Ok(output)
}

fn extract_expects(file: impl AsRef<Path>) -> Result<(Vec<Token>, String)> {
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

    Ok((tokens, expected))
}

pub fn run_test(file: impl AsRef<Path>) -> Result<()> {
    match extract_expects(file) {
        Ok((tokens, expected)) => match run(tokens) {
            Ok(output) => {
                if output == expected {
                    eprintln!("ok");
                    Ok(())
                } else {
                    eprintln!("failed");
                    eprintln!("    expected {:?}, got {:?}", expected, output);
                    Err(anyhow::anyhow!("Test failed"))
                }
            }
            Err(e) => {
                eprintln!("failed");
                eprintln!("    runtime error: {}", e);
                Err(anyhow::anyhow!("Test failed"))
            }
        },
        Err(e) => {
            eprintln!("failed");
            eprintln!("    parsing error: {}", e);
            Err(anyhow::anyhow!("Test failed"))
        }
    }
}
