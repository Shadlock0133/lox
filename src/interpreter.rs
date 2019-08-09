use crate::{
    syntax::*,
    tokens::{Token, TokenType, Value},
    visitor::*,
};
use std::fmt;

pub struct Interpreter;

impl Interpreter {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug)]
pub struct RuntimeError(Token, String);

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n[line {}]", self.1, self.0.line)
    }
}

impl std::error::Error for RuntimeError {}

impl Visitor<Expr, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, expr: &mut Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Binary(b) => b.accept(self),
            Expr::Grouping(b) => b.accept(self),
            Expr::Literal(b) => b.accept(self),
            Expr::Unary(b) => b.accept(self),
        }
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
                _ => Err(RuntimeError(
                    op.clone(),
                    "Operands must be a numbers.".into(),
                )),
            }
        }
        let left = (*t.1).accept(self)?;
        let right = (*t.2).accept(self)?;

        match t.0.type_ {
            TokenType::Plus => match (left, right) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
                _ => Err(RuntimeError(
                    t.0.clone(),
                    "Operands must be two numbers or two strings".into(),
                )),
            },
            TokenType::Minus => num_op(&t.0, left, right, |l, r| Value::Number(l - r)),
            TokenType::Star => num_op(&t.0, left, right, |l, r| Value::Number(l * r)),
            TokenType::Slash if right == Value::Number(0.0) => {
                Err(RuntimeError(t.0.clone(), "Can't divide by zero".into()))
            }
            TokenType::Slash => num_op(&t.0, left, right, |l, r| Value::Number(l / r)),

            TokenType::Greater => num_op(&t.0, left, right, |l, r| Value::Bool(l > r)),
            TokenType::GreaterEqual => num_op(&t.0, left, right, |l, r| Value::Bool(l >= r)),
            TokenType::Less => num_op(&t.0, left, right, |l, r| Value::Bool(l < r)),
            TokenType::LessEqual => num_op(&t.0, left, right, |l, r| Value::Bool(l <= r)),

            TokenType::EqualEqual => Ok(Value::Bool(left == right)),
            TokenType::BangEqual => Ok(Value::Bool(left != right)),
            _ => unimplemented!(),
        }
    }
}

impl Visitor<Literal, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Literal) -> Result<Value, RuntimeError> {
        Ok(t.0.clone())
    }
}

impl Visitor<Grouping, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Grouping) -> Result<Value, RuntimeError> {
        (*t.0).accept(self)
    }
}

impl Visitor<Unary, Result<Value, RuntimeError>> for Interpreter {
    fn visit(&mut self, t: &mut Unary) -> Result<Value, RuntimeError> {
        let value = (*t.1).accept(self)?;
        match t.0.type_ {
            TokenType::Minus => Ok(Value::Number(-value.as_number().ok_or_else(|| {
                RuntimeError(t.0.clone(), "Operand must be a number.".into())
            })?)),
            TokenType::Bang => Ok(Value::Bool(!value.is_truthy())),
            _ => Err(RuntimeError(
                t.0.clone(),
                "Unary expression must contain - or !".into(),
            )),
        }
    }
}
