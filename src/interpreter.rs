use crate::{
    ast::*,
    environment::Environment,
    errors::{RuntimeError, RuntimeResult},
    tokens::{Token, TokenType},
    types::{Class, Fun, Instance, NativeFunction, Value},
};
use core::fmt;
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    sync::{Arc, RwLock},
    time::Instant,
};

pub struct Interpreter<'a> {
    start_time: Instant,
    output: Box<dyn Write + 'a>,
    pub global: Environment,
    current: Environment,
    pub locals: HashMap<Expr, usize>,
}

impl fmt::Debug for Interpreter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interpreter")
            .field("start_time", &self.start_time)
            .field("output", &"Box<dyn Write>")
            .field("global", &self.global)
            .field("current", &self.current)
            .field("locals", &self.locals)
            .finish()
    }
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
            Value::fun(0, |_, _| {
                Err(RuntimeError::new(None, "Explicit panic"))
            }),
        );

        let current = global.clone();
        Self {
            start_time: Instant::now(),
            output: Box::new(output),
            global,
            current,
            locals: HashMap::new(),
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

    fn lookup_variable(
        &self,
        name: &Token,
        expr: &Expr,
    ) -> RuntimeResult<Value> {
        match self.locals.get(expr) {
            Some(&distance) => self.current.get_at(distance, name),
            None => self.global.get(name),
        }
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
                        _ => Err(RuntimeError::new(
                            Some(op),
                            "Operands must be numbers.",
                        )),
                    }
                }

                let left = self.visit_expr(&mut *left)?;

                match op.type_ {
                    TokenType::Or if left.is_truthy() => return Ok(left),
                    TokenType::Or if !left.is_truthy() => {
                        return self.visit_expr(&mut *right)
                    }
                    TokenType::And if !left.is_truthy() => return Ok(left),
                    TokenType::And if left.is_truthy() => {
                        return self.visit_expr(&mut *right)
                    }
                    _ => (),
                }

                let right = self.visit_expr(&mut *right)?;

                match op.type_ {
                    TokenType::Plus => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            Ok(Value::Number(l + r))
                        }
                        (Value::String(l), Value::String(r)) => {
                            Ok(Value::String(l + &r))
                        }
                        _ => Err(RuntimeError::new(
                            Some(op),
                            "Operands must be two numbers or two strings.",
                        )),
                    },
                    TokenType::Minus => {
                        num_op(op, left, right, |l, r| Value::Number(l - r))
                    }
                    TokenType::Star => {
                        num_op(op, left, right, |l, r| Value::Number(l * r))
                    }
                    // TokenType::Slash if right == Value::Number(0.0) => Err(
                    //     RuntimeError::new(Some(op), "Can't divide by zero."),
                    // ),
                    TokenType::Slash => {
                        num_op(op, left, right, |l, r| Value::Number(l / r))
                    }

                    TokenType::Greater => {
                        num_op(op, left, right, |l, r| Value::Bool(l > r))
                    }
                    TokenType::GreaterEqual => {
                        num_op(op, left, right, |l, r| Value::Bool(l >= r))
                    }
                    TokenType::Less => {
                        num_op(op, left, right, |l, r| Value::Bool(l < r))
                    }
                    TokenType::LessEqual => {
                        num_op(op, left, right, |l, r| Value::Bool(l <= r))
                    }

                    TokenType::EqualEqual => Ok(Value::Bool(left == right)),
                    TokenType::BangEqual => Ok(Value::Bool(left != right)),
                    _ => Err(RuntimeError::new(
                        Some(op),
                        "Invalid binary operator.",
                    )),
                }
            }

            Expr::Call {
                callee,
                right_paren,
                arguments,
            } => {
                let callee = self.visit_expr(callee)?;
                let mut arguments: Vec<Value> = arguments
                    .iter_mut()
                    .map(|e| self.visit_expr(e))
                    .collect::<Result<_, _>>()?;

                match callee {
                    Value::Fun(mut f) if f.arity() == arguments.len() => {
                        f.call(self, &mut arguments)
                    }
                    Value::Fun(f) => Err(RuntimeError::new(
                        Some(right_paren),
                        format!(
                            "Expected {} arguments but got {}.",
                            f.arity(),
                            arguments.len()
                        ),
                    )),
                    Value::Class(class) if arguments.is_empty() => {
                        Ok(Value::Instance(Arc::new(RwLock::new(
                            Instance::new(class),
                        ))))
                    }
                    _ => Err(RuntimeError::new(
                        Some(right_paren),
                        "Can only call functions and classes.",
                    )),
                }
            }

            Expr::Get { object, name } => {
                let object = self.visit_expr(object)?;
                if let Value::Instance(instance) = object {
                    instance.read().unwrap().get(name)
                } else {
                    Err(RuntimeError::new(
                        Some(name),
                        "Only instances have properties.",
                    ))
                }
            }

            Expr::Grouping { expr } => self.visit_expr(expr),

            Expr::Literal { value } => Ok(value.clone()),

            Expr::Set {
                object,
                name,
                value,
            } => {
                let object = self.visit_expr(object)?;
                if let Value::Instance(instance) = object {
                    let value = self.visit_expr(value)?;
                    instance.write().unwrap().set(name, value.clone());
                    Ok(value)
                } else {
                    Err(RuntimeError::new(
                        Some(name),
                        "Only instances have fields.",
                    ))
                }
            }

            Expr::Unary { op, right } => {
                let value = self.visit_expr(&mut *right)?;
                Ok(match op.type_ {
                    TokenType::Minus => {
                        let value = value.as_number().ok_or_else(|| {
                            RuntimeError::new(
                                Some(op),
                                "Operand must be a number.",
                            )
                        })?;
                        Value::Number(-value)
                    }
                    TokenType::Bang => Value::Bool(!value.is_truthy()),
                    _ => {
                        return Err(RuntimeError::new(
                            Some(op),
                            "Unary expression must contain '-' or '!'.",
                        ))
                    }
                })
            }

            Expr::Variable { name } => {
                self.lookup_variable(&name.clone(), expr)
            }
        }
    }

    fn visit_stmt(&mut self, stmt: &mut Stmt) -> RuntimeResult<()> {
        match stmt {
            Stmt::Block { statements } => {
                self.execute_block(statements, self.current.enclose())
            }

            Stmt::Class {
                name,
                methods: stmt_methods,
            } => {
                self.current.define(name.lexeme.clone(), Value::Nil);

                let mut methods = BTreeMap::new();
                for method in stmt_methods {
                    let function = NativeFunction {
                        name: Box::new(method.name.clone()),
                        params: method.params.clone(),
                        body: method.body.clone(),
                        closure: self.current.clone(),
                    };
                    methods.insert(method.name.lexeme.clone(), function);
                }

                let class = Class::new(name.lexeme.clone(), methods);
                self.current
                    .define(name.lexeme.clone(), Value::Class(class));
                Ok(())
            }

            Stmt::Expression { expr } => self.visit_expr(expr).map(drop),

            Stmt::Function(Function { name, params, body }) => {
                let closure = self.current.clone();
                let function = Value::Fun(Fun::Native(NativeFunction {
                    name: Box::new(name.clone()),
                    body: body.clone(),
                    params: params.clone(),
                    closure,
                }));
                self.current.define(name.lexeme.clone(), function);
                Ok(())
            }

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

            Stmt::Return { keyword: _, value } => Err(RuntimeError::Return(
                value
                    .as_mut()
                    .map(|e| self.visit_expr(e))
                    .transpose()?
                    .unwrap_or(Value::Nil),
            )),

            Stmt::Var { name, init } => {
                let value = init
                    .as_mut()
                    .map(|e| self.visit_expr(e))
                    .transpose()?
                    .unwrap_or(Value::Nil);
                self.current.define(name.lexeme.clone(), value);
                Ok(())
            }

            Stmt::While { condition, body } => {
                while self.visit_expr(condition)?.is_truthy() {
                    self.visit_stmt(body)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests;
