use crate::{visitor::*, syntax::*, tokens::{Value, TokenType}};

pub struct Interpreter;

#[derive(Debug)]
pub struct InterpretError;

impl Visitor<Expr, Result<Value, InterpretError>> for Interpreter {
    fn visit(&mut self, expr: &mut Expr) -> Result<Value, InterpretError> {
        match expr {
            Expr::Binary(b) => b.accept(self),
            Expr::Grouping(b) => b.accept(self),
            Expr::Literal(b) => b.accept(self),
            Expr::Unary(b) => b.accept(self),
        }
    }
}

impl Visitor<Binary, Result<Value, InterpretError>> for Interpreter {
    fn visit(&mut self, t: &mut Binary) -> Result<Value, InterpretError> {
        unimplemented!();
        // let op = match t.1.type_ {
        //     TokenType::Plus => |a, b| Value::Number(a + b),
        // };
        // op((*t.0).accept(&mut Self), (*t.2).accept(&mut Self))
    }
}

impl Visitor<Literal, Result<Value, InterpretError>> for Interpreter {
    fn visit(&mut self, t: &mut Literal) -> Result<Value, InterpretError> {
        Ok(t.0.clone())
    }
}

impl Visitor<Grouping, Result<Value, InterpretError>> for Interpreter {
    fn visit(&mut self, t: &mut Grouping) -> Result<Value, InterpretError> {
        (*t.0).accept(self)
    }
}

impl Visitor<Unary, Result<Value, InterpretError>> for Interpreter {
    fn visit(&mut self, t: &mut Unary) -> Result<Value, InterpretError> {
        let value = (*t.1).accept(self)?;
        match t.0.type_ {
            TokenType::Minus => Ok(Value::Number(-value.as_number().ok_or(InterpretError)?)),
            TokenType::Bang => Ok(Value::Bool(value.is_truthy())),
            _ => Err(InterpretError),
        }
    }
}