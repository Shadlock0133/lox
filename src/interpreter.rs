use crate::{
    ast::*,
    // environment::Environment,
    errors::{RuntimeError, RuntimeResult},
    tokens::{Token, TokenType},
    types::Value,
};
use std::{io::Write, time::Instant};

pub struct Interpreter {
    start_time: Instant,
    output: Box<dyn Write>,
    // pub global: Environment,
    // current: Environment,
}

impl Interpreter {
    pub fn new<W: Write + 'static>(output: W) -> Self {
        // let global = Environment::new();

        // global.define(
        //     "clock".into(),
        //     Value::fun(0, |inter, _| {
        //         let dur = inter.start_time.elapsed();
        //         Value::Number(dur.as_nanos() as f64 * 1e-9)
        //     }),
        // );

        // let current = Rc::clone(&global);
        Self {
            start_time: Instant::now(),
            output: Box::new(output),
            // global,
            // current,
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
        // environment: Environment,
    ) -> RuntimeResult<()> {
        // let previous = Rc::clone(&self.current);
        let result = (|| {
            // self.current = environment;
            for statement in statements {
                self.visit_stmt(statement)?;
            }
            Ok(())
        })();
        // self.current = previous;
        result
    }

    fn visit_expr(&mut self, expr: &mut Expr) -> RuntimeResult<Value> {
        match expr {
            Expr::Assign { name, value } => todo!("visit_expr"),

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
                callee,
                right_paren,
                arguments,
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

            Expr::Variable { name } => todo!("visit_expr"),
        }
    }

    fn visit_stmt(&mut self, stmt: &mut Stmt) -> RuntimeResult<()> {
        match stmt {
            Stmt::Block { statements } => todo!("visit_stmt"),

            Stmt::Expression { expr } => todo!("visit_stmt"),

            Stmt::Function { name, params, body } => todo!("visit_stmt"),

            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => todo!("visit_stmt"),

            Stmt::PrintStmt { expr } => {
                let value = self.visit_expr(expr)?;
                writeln!(self.output, "{}", value)
                    .map_err(|e| RuntimeError::new(None, e.to_string()))
            }

            Stmt::Return { keyword, value } => todo!("visit_stmt"),

            Stmt::Var { name, init } => todo!("visit_stmt"),

            Stmt::While { condition, body } => todo!("visit_stmt"),
        }
    }
}
