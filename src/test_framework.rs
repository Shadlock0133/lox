use std::{fs, path::Path};

use crate::{
    errors, interpreter::*, parser::*, resolver::Resolver, tokenizer::*,
    tokens::*,
};

use anyhow::Result;

const SKIP: &[&str] = &["benchmark", "expressions", "limit", "scanning"];

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
            let res = run_test(&path);
            match res {
                Ok(_) => *passes += 1,
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
    run_tests_rec(dir.as_ref(), dir.as_ref(), &mut passes, &mut fails)?;
    eprintln!("successes: {}, fails: {}", passes, fails);
    if fails == 0 {
        Ok(())
    } else {
        Err(anyhow::anyhow!("tests failed"))
    }
}

fn run(tokens: Vec<Token>, output: &mut Vec<u8>) -> Result<()> {
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse()?;

    let mut interpreter = Interpreter::new(output);

    let mut resolver = Resolver::new(&mut interpreter.locals);
    resolver.resolve(&program)?;

    interpreter.interpret(&mut program)?;

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

fn tokenize(file: impl AsRef<Path>) -> Result<Vec<Token>> {
    let source = fs::read_to_string(file)?;
    let tokens: Vec<_> = Tokenizer::new(&source)
        // .inspect(|x| eprintln!("{:?}", x))
        .collect::<std::result::Result<_, errors::TokenizerError>>()?;
    Ok(tokens)
}

pub fn run_test(file: impl AsRef<Path>) -> Result<()> {
    let mut tokens = match tokenize(file) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("failed");
            eprintln!("    tokenize error: {}", e);
            return Err(e);
        }
    };

    let expected = extract_expects(&tokens);

    // Remove comments and whitespaces
    tokens.retain(|x| !x.can_skip());

    let mut output = vec![];
    let res = run(tokens, &mut output);
    let output = match String::from_utf8(output) {
        Ok(string) => string,
        Err(e) => {
            eprintln!("failed");
            eprintln!("    string error: {}", e);
            return Err(e.into());
        }
    };
    let unimplemented_class_syntax =
        ["'<'", "'this'", "'super'", "initializer"];
    match (res, expected.runtime_error) {
        (Ok(()), None) => {
            if output == expected.output {
                eprintln!("ok");
                Ok(())
            } else {
                eprintln!("failed");
                eprintln!(
                    "    expected output {:?},\n    got {:?}",
                    expected.output, output
                );
                Err(anyhow::anyhow!("Test failed"))
            }
        }
        (Err(e), Some(re)) => {
            let e = e.to_string();
            if e.ends_with(&re) {
                eprintln!("ok");
                Ok(())
            } else {
                eprintln!("failed");
                if unimplemented_class_syntax.iter().any(|x| e.contains(x)) {
                    eprintln!("    unimplemented class syntax");
                } else {
                    eprintln!("    expected error {:?},\n    got {:?}", e, re);
                }
                Err(anyhow::anyhow!("Test failed"))
            }
        }
        (Err(e), None) => {
            eprintln!("failed");
            let msg = e.to_string();
            if unimplemented_class_syntax.iter().any(|x| msg.contains(x)) {
                eprintln!("    unimplemented class syntax");
            } else {
                eprintln!("    unexpected runtime error: {}", e);
            }
            Err(anyhow::anyhow!("Test failed"))
        }
        (Ok(_), Some(re)) => {
            eprintln!("failed");
            eprintln!("    expected failure: {:?}", re);
            Err(anyhow::anyhow!("Test failed"))
        }
    }
}
