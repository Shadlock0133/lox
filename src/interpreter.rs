use crate::{
    ast::*,
    environment::Environment,
    errors::{RuntimeError, RuntimeResult},
    tokens::{Token, TokenType},
    types::Value,
};
use std::{io::Write, time::Instant};

pub struct Interpreter<'a> {
    start_time: Instant,
    output: Box<dyn Write + 'a>,
    pub global: Environment,
    current: Environment,
}

impl<'a> Interpreter<'a> {
    pub fn new<W: Write + 'a>(output: W) -> Self {
        let mut global = Environment::new();

        global.define(
            "clock".into(),
            Value::fun(0, |interpreter, _| {
                let dur = interpreter.start_time.elapsed();
                Ok(Value::Number(dur.as_nanos() as f64 * 1e-9))
            }),
        );

        global.define(
            "panic".into(),
            Value::fun(0, |_, _| Err(RuntimeError::new(None, "explicit panic"))),
        );

        let current = global.clone();
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
                self.visit_stmt(statement)?;
            }
            Ok(())
        })();

        match result {
            Err(RuntimeError::Error(_)) => result,
            _ => Ok(()),
        }
    }

    pub fn execute_block(
        &mut self,
        statements: &mut [Stmt],
        environment: Environment,
    ) -> RuntimeResult<()> {
        let previous = self.current.clone();
        let result = (|| {
            self.current = environment;
            for statement in statements {
                self.visit_stmt(statement)?;
            }
            Ok(())
        })();
        self.current = previous;
        result
    }

    fn visit_expr(&mut self, expr: &mut Expr) -> RuntimeResult<Value> {
        match expr {
            Expr::Assign { name, value } => {
                let value = self.visit_expr(value)?;
                self.current.assign(name, value.clone())?;
                Ok(value)
            }

            Expr::Binary { op, left, right } => {
                fn num_op<F: Fn(f64, f64) -> Value>(
                    op: &Token,
                    l: Value,
                    r: Value,
                    f: F,
                ) -> RuntimeResult<Value> {
                    match (l, r) {
                        (Value::Number(l), Value::Number(r)) => Ok(f(l, r)),
                        _ => Err(RuntimeError::new(Some(op), "Operands must be a numbers.")),
                    }
                }

                let left = self.visit_expr(&mut *left)?;

                match op.type_ {
                    TokenType::Or if left.is_truthy() => return Ok(left),
                    TokenType::Or if !left.is_truthy() => return self.visit_expr(&mut *right),
                    TokenType::And if !left.is_truthy() => return Ok(left),
                    TokenType::And if left.is_truthy() => return self.visit_expr(&mut *right),
                    _ => (),
                }

                let right = self.visit_expr(&mut *right)?;

                match op.type_ {
                    TokenType::Plus => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
                        (Value::String(l), r) => Ok(Value::String(l + &r.to_string())),
                        (l, Value::String(r)) => Ok(Value::String(l.to_string() + &r)),
                        _ => Err(RuntimeError::new(
                            Some(op),
                            "Operands must begin with a string or be two numbers.",
                        )),
                    },
                    TokenType::Minus => num_op(op, left, right, |l, r| Value::Number(l - r)),
                    TokenType::Star => num_op(op, left, right, |l, r| Value::Number(l * r)),
                    TokenType::Slash if right == Value::Number(0.0) => {
                        Err(RuntimeError::new(Some(op), "Can't divide by zero."))
                    }
                    TokenType::Slash => num_op(op, left, right, |l, r| Value::Number(l / r)),

                    TokenType::Greater => num_op(op, left, right, |l, r| Value::Bool(l > r)),
                    TokenType::GreaterEqual => num_op(op, left, right, |l, r| Value::Bool(l >= r)),
                    TokenType::Less => num_op(op, left, right, |l, r| Value::Bool(l < r)),
                    TokenType::LessEqual => num_op(op, left, right, |l, r| Value::Bool(l <= r)),

                    TokenType::EqualEqual => Ok(Value::Bool(left == right)),
                    TokenType::BangEqual => Ok(Value::Bool(left != right)),
                    _ => Err(RuntimeError::new(Some(op), "Invalid binary operator.")),
                }
            }

            Expr::Call {
                callee: _,
                right_paren: _,
                arguments: _,
            } => todo!("visit_expr"),

            Expr::Grouping { expr } => self.visit_expr(expr),

            Expr::Literal { value } => Ok(value.clone()),

            Expr::Unary { op, right } => {
                let value = self.visit_expr(&mut *right)?;
                Ok(match op.type_ {
                    TokenType::Minus => {
                        let value = value.as_number().ok_or_else(|| {
                            RuntimeError::new(Some(op), "Operand must be a number.")
                        })?;
                        Value::Number(-value)
                    }
                    TokenType::Bang => Value::Bool(!value.is_truthy()),
                    _ => Err(RuntimeError::new(
                        Some(op),
                        "Unary expression must contain '-' or '!'.",
                    ))?,
                })
            }

            Expr::Variable { name } => self.current.get(name),
        }
    }

    fn visit_stmt(&mut self, stmt: &mut Stmt) -> RuntimeResult<()> {
        match stmt {
            Stmt::Block { statements } => self.execute_block(statements, self.current.enclose()),

            Stmt::Expression { expr } => self.visit_expr(expr).map(drop),

            Stmt::Function {
                name: _,
                params: _,
                body: _,
            } => todo!("visit_stmt"),

            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.visit_expr(condition)?.is_truthy() {
                    self.visit_stmt(then_branch)?;
                } else if let Some(else_branch) = else_branch {
                    self.visit_stmt(else_branch)?;
                }
                Ok(())
            }

            Stmt::PrintStmt { expr } => {
                let value = self.visit_expr(expr)?;
                writeln!(self.output, "{}", value)
                    .map_err(|e| RuntimeError::new(None, e.to_string()))
            }

            Stmt::Return {
                keyword: _,
                value: _,
            } => todo!("visit_stmt"),

            Stmt::Var { name, init } => {
                let value = init
                    .as_mut()
                    .map(|e| self.visit_expr(e))
                    .transpose()?
                    .unwrap_or(Value::Nil);
                self.current.define(name.lexeme.clone(), value);
                Ok(())
            }

            Stmt::While {
                condition: _,
                body: _,
            } => todo!("visit_stmt"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_type(t: TokenType) -> Token {
        Token {
            type_: t,
            lexeme: "".into(),
            literal: None,
            line: 0,
        }
    }

    fn identifier(name: &str) -> Token {
        Token {
            type_: TokenType::Identifier,
            lexeme: name.into(),
            literal: None,
            line: 0,
        }
    }

    #[test]
    fn exprs() {
        let expr = |mut x| Interpreter::new(vec![]).visit_expr(&mut x).unwrap();

        assert_eq!(
            expr(Expr::binary(
                from_type(TokenType::Plus),
                Expr::literal(Value::Number(1.0)),
                Expr::literal(Value::Number(2.0))
            )),
            Value::Number(3.0)
        );
        assert_eq!(
            expr(Expr::binary(
                from_type(TokenType::Plus),
                Expr::literal(Value::String("foo".into())),
                Expr::literal(Value::String("bar".into()))
            )),
            Value::String("foobar".into())
        );
    }

    #[test]
    fn print() {
        let run = |x: &mut [_]| {
            let mut v = vec![];
            Interpreter::new(&mut v).interpret(x).unwrap();
            v
        };

        assert_eq!(
            run(&mut [Stmt::print(Expr::literal(Value::String("one".into())))]),
            b"one\n"
        );

        assert_eq!(
            run(&mut [Stmt::print(Expr::literal(Value::Bool(true)))]),
            b"true\n"
        );

        assert_eq!(
            run(&mut [Stmt::print(Expr::binary(
                from_type(TokenType::Plus),
                Expr::literal(Value::Number(1.0)),
                Expr::literal(Value::Number(2.0)),
            ))]),
            b"3\n"
        );

        assert_eq!(
            run(&mut [
                Stmt::var(identifier("a"), Some(Expr::literal(Value::Number(1.0)))),
                Stmt::print(Expr::variable(identifier("a")))
            ]),
            b"1\n"
        );

        assert_eq!(
            run(&mut [
                Stmt::var(identifier("a"), Some(Expr::literal(Value::Number(1.0)))),
                Stmt::print(Expr::assign(
                    identifier("a"),
                    Expr::literal(Value::Number(2.0))
                ))
            ]),
            b"2\n"
        );

        assert_eq!(
            run(&mut [
                Stmt::var(identifier("a"), Some(Expr::literal(Value::Number(1.0)))),
                Stmt::var(identifier("b"), Some(Expr::literal(Value::Number(1.0)))),
                Stmt::print(Expr::variable(identifier("a"))),
                Stmt::print(Expr::variable(identifier("b"))),
                Stmt::block(vec![
                    Stmt::var(identifier("a"), Some(Expr::literal(Value::Number(2.0)))),
                    Stmt::expression(Expr::assign(
                        identifier("b"),
                        Expr::literal(Value::Number(2.0))
                    )),
                    Stmt::print(Expr::variable(identifier("a"))),
                    Stmt::print(Expr::variable(identifier("b"))),
                ]),
                Stmt::print(Expr::variable(identifier("a"))),
                Stmt::print(Expr::variable(identifier("b"))),
            ]),
            b"1\n1\n2\n2\n1\n2\n"
        );

        assert_eq!(
            run(&mut [
                Stmt::var(identifier("a"), Some(Expr::literal(Value::Number(1.0)))),
                Stmt::block(vec![
                    Stmt::var(
                        identifier("a"),
                        Some(Expr::binary(
                            from_type(TokenType::Plus),
                            Expr::literal(Value::Number(2.0)),
                            Expr::variable(identifier("a")),
                        ))
                    ),
                    Stmt::print(Expr::variable(identifier("a"))),
                ]),
            ]),
            b"3\n"
        );
    }
}
