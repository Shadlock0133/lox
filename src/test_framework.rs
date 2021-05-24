use std::{fs, io, path::Path, string::FromUtf8Error, time::Instant};

use crate::{
    errors::{ParseError, ResolveError, RuntimeError, TokenizerError},
    interpreter::*,
    parser::*,
    resolver::Resolver,
    tokenizer::*,
    tokens::*,
};

use anyhow::Result;

macro_rules! term {
    (ESC) => {
        "\x1b["
    };
    (GREEN) => {
        concat!(term!(ESC), "32m")
    };
    (RED) => {
        concat!(term!(ESC), "31m")
    };
    (RESET) => {
        concat!(term!(ESC), "m")
    };
}
const OK: &str = concat!(term!(GREEN), "ok", term!(RESET));
const FAILED: &str = concat!(term!(RED), "FAILED", term!(RESET));

const SKIP: &[&str] = &["benchmark", "expressions", "limit", "scanning"];
const UNIMPLEMENTED_CLASS_SYNTAX: &[&str] = &["'<'", "'super'", "initializer"];

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
            let res = run_test_without_prefix(prefix.as_ref(), &path);
            match res {
                Ok(()) => *passes += 1,
                Err(_) => {
                    eprintln!("    in {}", path.display());
                    *fails += 1;
                }
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
    let timer = Instant::now();
    run_tests_rec(dir.as_ref(), dir.as_ref(), &mut passes, &mut fails)?;
    let result = if fails == 0 { OK } else { FAILED };
    eprintln!(
        "test result: {}. {} passed, {} failed; finished in {:?}",
        result,
        passes,
        fails,
        timer.elapsed()
    );
    if fails == 0 {
        Ok(())
    } else {
        Err(anyhow::anyhow!("tests failed"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("Resolve error: {0}")]
    Resolve(#[from] ResolveError),
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),
}

fn run(tokens: Vec<Token>, output: &mut Vec<u8>) -> Result<(), RunError> {
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse()?;

    let mut interpreter = Interpreter::new(output);

    let mut resolver = Resolver::new(&mut interpreter.locals);
    resolver.resolve(&program)?;
    
    interpreter
        .interpret(&mut program)
        .map_err(|x| x.into_error())?;

    drop(interpreter);
    Ok(())
}

struct Expect {
    output: String,
    runtime_error: Option<String>,
}

// Extract expected output and/or errors
fn extract_expects(tokens: &[Token]) -> Expect {
    let runtime_error_direct = tokens
        .iter()
        .filter(|t| t.type_ == TokenType::Comment)
        .filter(|t| t.lexeme.contains("Error at"))
        .filter_map(|t| t.lexeme.trim().split_once(": ").map(|x| x.1))
        .next();

    let runtime_error_expect = tokens
        .iter()
        .filter_map(|t| {
            if t.type_ != TokenType::Comment {
                return None;
            }
            t.lexeme
                .trim()
                .trim_start_matches("// ")
                .strip_prefix("expect runtime error: ")
        })
        .next();

    if runtime_error_direct.and(runtime_error_expect).is_some() {
        panic!("both direct and expect errors");
    }
    let runtime_error = runtime_error_direct
        .xor(runtime_error_expect)
        .map(ToOwned::to_owned);

    let output: String = tokens
        .iter()
        .filter_map(|t| {
            if t.type_ != TokenType::Comment {
                return None;
            }
            t.lexeme
                .trim()
                .trim_start_matches("// ")
                .strip_prefix("expect: ")
        })
        .flat_map(|x| std::array::IntoIter::new([x, "\n"]))
        .collect();

    Expect {
        output,
        runtime_error,
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, TokenizerError> {
    Tokenizer::new(source).collect()
}

#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Io error: {0}")]
    Io(#[from] io::Error),
    #[error("Non Utf8 output: {0}")]
    NonUtf8Output(#[from] FromUtf8Error),
    #[error("Tokenizer error: expected {0:?}, got {1}")]
    Tokenizer(Option<String>, TokenizerError),
    #[error("Runtime error: expected {0:?}, got {1}")]
    Run(Option<String>, RunError),
    #[error("Missing run error: {0:?}")]
    MissingRunError(String),
    #[error("Wrong output: expected {0:?}, got {1:?}")]
    WrongOutput(String, String),
}

pub fn run_test(path: impl AsRef<Path>) -> Result<()> {
    run_test_without_prefix("", path)
}

fn run_test_without_prefix(
    prefix: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<()> {
    eprint!(
        "test {} ... ",
        path.as_ref()
            .strip_prefix(prefix.as_ref())
            .unwrap()
            .display()
    );
    let error = match test_handler(path) {
        Ok(()) => {
            eprintln!("{}", OK);
            return Ok(());
        }
        Err(e) => e,
    };
    eprintln!("{}", FAILED);
    match &error {
        TestError::Tokenizer(Some(expected), got) => {
            eprintln!("    expected tokenize error: {:?}", expected);
            eprintln!("    got: {}", got);
        }
        TestError::Tokenizer(None, got) => {
            eprintln!("    tokenize error: {}", got)
        }
        TestError::Run(Some(expected), got) => {
            let msg = got.to_string();
            if UNIMPLEMENTED_CLASS_SYNTAX.iter().any(|x| msg.contains(x)) {
                eprintln!("    unimplemented class syntax");
            } else {
                eprintln!("    expected error {:?}", expected);
                eprintln!("    got {}", got);
            }
        }
        TestError::Run(None, got) => {
            let msg = got.to_string();
            if UNIMPLEMENTED_CLASS_SYNTAX.iter().any(|x| msg.contains(x)) {
                eprintln!("    unimplemented class syntax");
            } else {
                eprintln!("    unexpected runtime error: {}", got);
            }
        }
        TestError::MissingRunError(got) => {
            eprintln!("    expected failure: {:?}", got)
        }
        TestError::WrongOutput(expected, got) => {
            eprintln!("    expected output: {:?}", expected);
            eprintln!("    got: {:?}", got);
        }
        TestError::Io(e) => eprintln!("    {:?}", e),
        TestError::NonUtf8Output(e) => eprintln!("    {:?}", e),
    }
    Err(error.into())
}

// Check for expected tokenize error on first line
// Can't use `extract_expects` because it takes already tokenized input
fn first_line_expect(source: &str) -> Option<String> {
    Tokenizer::new(&source)
        .next()
        // if there was an error on first token, it probably wasn't an expect
        .transpose()
        .ok()
        .flatten()
        // check if it's an expect
        .filter(|x| x.type_ == TokenType::Comment)
        .filter(|x| x.lexeme.contains("Error"))
        .as_ref()
        // extract expected message
        .and_then(|x| x.lexeme.trim().split_once(": ").map(|x| x.1))
        .map(ToOwned::to_owned)
}

fn test_handler(file: impl AsRef<Path>) -> Result<(), TestError> {
    let source = fs::read_to_string(file)?;
    let mut all_tokens = match tokenize(&source) {
        Ok(tokens) => tokens,
        Err(e) => {
            return match first_line_expect(&source) {
                Some(expected) if e.to_string().ends_with(&expected) => Ok(()),
                Some(expected) => Err(TestError::Tokenizer(Some(expected), e)),
                None => Err(TestError::Tokenizer(None, e)),
            };
        }
    };

    let expected = extract_expects(&all_tokens);

    // Removes comments and whitespaces
    all_tokens.retain(|x| !x.can_skip());
    let tokens = all_tokens;

    let mut output = vec![];
    let res = run(tokens, &mut output);
    let output = String::from_utf8(output)?;
    match (res, expected.runtime_error) {
        (Ok(()), None) if output == expected.output => Ok(()),
        (Ok(()), None) => Err(TestError::WrongOutput(expected.output, output)),
        (Err(e), Some(re)) if e.to_string().ends_with(&re) => Ok(()),
        (Err(e), Some(re)) => Err(TestError::Run(Some(re), e)),
        (Err(e), None) => Err(TestError::Run(None, e)),
        (Ok(_), Some(re)) => Err(TestError::MissingRunError(re)),
    }
}
