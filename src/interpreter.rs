use crate::{
    environment::Environment,
    errors::{RuntimeError, RuntimeResult},
    syntax::*,
    // tokens::{Token, TokenType},
    types::{Fun, Value},
};
use std::{cell::RefCell, io::Write, rc::Rc, time::Instant};

pub struct Interpreter {
    start_time: Instant,
    output: Box<dyn Write>,
    pub global: Environment,
    current: Environment,
}

impl Interpreter {
    pub fn new<W: Write + 'static>(output: W) -> Self {
        let global = Environment::new();

        global.define(
            "clock".into(),
            Value::fun(0, |inter, _| {
                let dur = inter.start_time.elapsed();
                Value::Number(dur.as_nanos() as f64 * 1e-9)
            }),
        );

        let current = Rc::clone(&global);
        Self {
            start_time: Instant::now(),
            output: Box::new(output),
            global,
            current,
        }
    }

    pub fn interpret(&mut self, statements: &mut [Stmt]) -> RuntimeResult<()> {
        let result = (|| {
            for statement in statements {
                // self.visit(statement)?;
            }
            Ok(())
        })();

        match result {
            Err(RuntimeError::Error(_, _)) => result,
            _ => Ok(()),
        }
    }

    pub fn execute_block(
        &mut self,
        statements: &mut [Stmt],
        environment: Environment,
    ) -> RuntimeResult<()> {
        let previous = Rc::clone(&self.current);
        let result = (|| {
            self.current = environment;
            for statement in statements {
                // self.visit(statement)?;
            }
            Ok(())
        })();
        self.current = previous;
        result
    }
}
