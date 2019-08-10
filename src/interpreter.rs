use crate::{
    environment::Environment,
    syntax::*,
    tokens::{Token, TokenType, Value},
    visitor::*,
};
use std::{
    cell::RefCell,
    fmt::{self, Write},
    rc::Rc,
};

pub struct Interpreter {
    output: String,
    environment: Rc<RefCell<Environment>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            environment: Environment::new(),
        }
    }

    pub fn interpret(&mut self, statements: Vec<Stmt>) -> Result<String, RuntimeError> {
        for mut statement in statements {
            self.visit(&mut statement)?;
        }
        Ok(std::mem::replace(&mut self.output, String::new()))
    }

    fn execute_block(&mut self, statements: &mut [Stmt], environment: Rc<RefCell<Environment>>) -> Result<(), RuntimeError> {
        let previous = Rc::clone(&self.environment);
        let result = (|| {
            self.environment = environment;
            for statement in statements {
                self.visit(statement)?;
            }
            Ok(())
        })();
        self.environment = previous;
        result
    }
}

#[derive(Debug)]
pub struct RuntimeError(Token, String);

impl RuntimeError {
    pub fn new<S: Into<String>>(token: &Token, message: S) -> Self {
        Self(token.clone(), message.into())
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n[line {}]", self.1, self.0.line)
    }
}

impl std::error::Error for RuntimeError {}

impl Visitor<Assign, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Assign) -> Result<Value, RuntimeError> {
        let value = self.visit(&mut *t.value)?;
        self.environment.borrow_mut().assign(&t.name, value.clone())?;
        Ok(value)
    }
}

impl Visitor<Binary, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Binary) -> Result<Value, RuntimeError> {
        fn num_op<F: Fn(f64, f64) -> Value>(
            op: &Token,
            l: Value,
            r: Value,
            f: F,
        ) -> Result<Value, RuntimeError> {
            match (l, r) {
                (Value::Number(l), Value::Number(r)) => Ok(f(l, r)),
                _ => Err(RuntimeError::new(&op, "Operands must be a numbers.")),
            }
        }

        let left = self.visit(&mut *t.left)?;

        match t.op.type_ {
            TokenType::Or if left.is_truthy() => return Ok(left),
            TokenType::Or if !left.is_truthy() => return self.visit(&mut *t.right),
            TokenType::And if !left.is_truthy() => return Ok(left),
            TokenType::And if left.is_truthy() => return self.visit(&mut *t.right),
            _ => (),
        }

        let right = self.visit(&mut *t.right)?;

        match t.op.type_ {
            TokenType::Plus => match (left, right) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
                _ => Err(RuntimeError::new(
                    &t.op,
                    "Operands must be two numbers or two strings",
                )),
            },
            TokenType::Minus => num_op(&t.op, left, right, |l, r| Value::Number(l - r)),
            TokenType::Star => num_op(&t.op, left, right, |l, r| Value::Number(l * r)),
            TokenType::Slash if right == Value::Number(0.0) => {
                Err(RuntimeError::new(&t.op, "Can't divide by zero"))
            }
            TokenType::Slash => num_op(&t.op, left, right, |l, r| Value::Number(l / r)),

            TokenType::Greater => num_op(&t.op, left, right, |l, r| Value::Bool(l > r)),
            TokenType::GreaterEqual => num_op(&t.op, left, right, |l, r| Value::Bool(l >= r)),
            TokenType::Less => num_op(&t.op, left, right, |l, r| Value::Bool(l < r)),
            TokenType::LessEqual => num_op(&t.op, left, right, |l, r| Value::Bool(l <= r)),

            TokenType::EqualEqual => Ok(Value::Bool(left == right)),
            TokenType::BangEqual => Ok(Value::Bool(left != right)),
            _ => unimplemented!(),
        }
    }
}

impl Visitor<Literal, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Literal) -> Result<Value, RuntimeError> {
        Ok(t.value.clone())
    }
}

impl Visitor<Grouping, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Grouping) -> Result<Value, RuntimeError> {
        self.visit(&mut *t.expr)
    }
}

impl Visitor<Unary, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Unary) -> Result<Value, RuntimeError> {
        let value = self.visit(&mut *t.right)?;
        match t.op.type_ {
            TokenType::Minus => {
                Ok(Value::Number(-value.as_number().ok_or_else(|| {
                    RuntimeError::new(&t.op, "Operand must be a number.")
                })?))
            }
            TokenType::Bang => Ok(Value::Bool(!value.is_truthy())),
            _ => Err(RuntimeError::new(
                &t.op,
                "Unary expression must contain - or !",
            )),
        }
    }
}

impl Visitor<Variable, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Variable) -> Result<Value, RuntimeError> {
        self.environment.borrow().get(&t.name)
    }
}

impl Visitor<Block, Result<(), RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Block) -> Result<(), RuntimeError> {
        self.execute_block(&mut t.statements, Environment::from_enclosing(&self.environment))
    }
}

impl Visitor<Expression, Result<(), RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Expression) -> Result<(), RuntimeError> {
        self.visit(&mut t.expr)?;
        Ok(())
    }
}

impl Visitor<If, Result<(), RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut If) -> Result<(), RuntimeError> {
        if self.visit(&mut t.condition)?.is_truthy() {
            self.visit(&mut *t.then_branch)?;
        } else if let Some(else_branch) = &mut t.else_branch {
            self.visit(&mut **else_branch)?;
        }
        Ok(())
    }
}

impl Visitor<PrintStmt, Result<(), RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut PrintStmt) -> Result<(), RuntimeError> {
        let value = self.visit(&mut t.expr)?;
        let _ = writeln!(self.output, "{}", value);
        Ok(())
    }
}

impl Visitor<Var, Result<(), RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Var) -> Result<(), RuntimeError> {
        let value = match &mut t.init {
            Some(expr) => self.visit(expr)?,
            None => Value::Nil,
        };
        self.environment.borrow_mut().define(t.name.lexeme.clone(), value);
        Ok(())
    }
}
