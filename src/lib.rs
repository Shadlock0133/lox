pub mod clox;
pub mod jlox;

use std::{fs, path::Path};

use crate::{
    clox::compiler::compile,
    jlox::{
        errors::TokenizerError, interpreter::*, parser::*, resolver::Resolver,
        tokenizer::*, tokens::*,
    },
};

use anyhow::Result;
use clox::vm::{Vm, VmState};
use jlox::test_framework;

pub trait Lox {
    fn interpret(&mut self, source: String) -> Result<()>;

    fn run_file<P: AsRef<Path>>(&mut self, file: P) -> Result<()> {
        let script = fs::read_to_string(file)?;
        self.interpret(script)?;
        Ok(())
    }

    fn run_repl(&mut self) -> Result<()> {
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
                    let res = self.interpret(input);
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
}

pub struct JLox {
    interpreter: Interpreter<'static>,
}

impl JLox {
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(std::io::stdout()),
        }
    }

    pub fn run_test<A: AsRef<Path>>(path: A) -> Result<()> {
        test_framework::run_test(path)
    }

    pub fn run_tests<A: AsRef<Path>>(path: A) -> Result<()> {
        test_framework::run_tests(path)
    }
}

impl Lox for JLox {
    fn interpret(&mut self, source: String) -> Result<()> {
        let tokenizer = Tokenizer::new(&source);
        let tokens: Vec<Token> = tokenizer
            .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
            .collect::<std::result::Result<_, TokenizerError>>()?;

        let mut parser = Parser::new(tokens);
        let mut program = parser.parse()?;

        let mut resolver = Resolver::new(&mut self.interpreter.locals);
        resolver.resolve(&program)?;

        self.interpreter.interpret(&mut program)?;

        Ok(())
    }
}

pub struct CLox {
    state: VmState,
    debug: bool,
}

impl CLox {
    pub fn new(debug: bool) -> Self {
        Self {
            state: Default::default(),
            debug,
        }
    }
}

impl Lox for CLox {
    fn interpret(&mut self, source: String) -> Result<()> {
        let chunk = compile(&source)?;
        let mut vm = Vm::new(&chunk, &mut self.state);
        vm.interpret(self.debug)?;
        Ok(())
    }
}
