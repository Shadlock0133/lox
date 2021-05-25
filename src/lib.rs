pub mod jlox;

use std::{fs, path::Path};

use crate::jlox::{
    errors::TokenizerError, interpreter::*, parser::*, resolver::Resolver,
    tokenizer::*, tokens::*,
};

use anyhow::Result;

pub struct JLox {
    interpreter: Interpreter<'static>,
}

impl JLox {
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
            .collect::<std::result::Result<_, TokenizerError>>()?;

        let mut parser = Parser::new(tokens);
        let mut program = parser.parse()?;

        let mut resolver = Resolver::new(&mut self.interpreter.locals);
        resolver.resolve(&program)?;

        self.interpreter.interpret(&mut program)?;

        Ok(())
    }
}
