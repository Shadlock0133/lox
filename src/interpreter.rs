use crate::{
    ast::*,
    environment::Environment,
    errors::{ControlFlow, RuntimeError, RuntimeResult},
    tokens::{Token, TokenType},
    types::{Class, Fun, Instance, LoxFunction, Value, ValueRef},
};
use core::fmt;
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
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
            ValueRef::fun(0, |interpreter, _| {
                let dur = interpreter.start_time.elapsed();
                Ok(ValueRef::from_value(Value::Number(
                    dur.as_nanos() as f64 * 1e-9,
                )))
            }),
        );

        global.define(
            "panic".into(),
            ValueRef::fun(0, |_, _| {
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
            Err(ControlFlow::Error(_)) => result,
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
    ) -> RuntimeResult<ValueRef> {
        let get = self.locals.get(expr);
        match get {
            Some(&distance) => self.current.get_at(distance, name),
            None => self.global.get(name),
        }
    }

    fn visit_expr(&mut self, expr: &mut Expr) -> RuntimeResult<ValueRef> {
        match expr {
            Expr::Assign { name, value } => {
                let name = name.clone();
                let value = self.visit_expr(value)?;
                match self.locals.get(&expr) {
                    Some(&distance) => self.current.assign_at(
                        distance,
                        &name,
                        value.clone(),
                    )?,
                    None => self.global.assign(&name, value.clone())?,
                }
                Ok(value)
            }

            Expr::Binary { op, left, right } => {
                fn num_op<F: Fn(f64, f64) -> ValueRef>(
                    op: &Token,
                    l: ValueRef,
                    r: ValueRef,
                    f: F,
                ) -> RuntimeResult<ValueRef> {
                    match (l.value(), r.value()) {
                        (Value::Number(l), Value::Number(r)) => Ok(f(l, r)),
                        _ => Err(RuntimeError::new(
                            Some(op),
                            "Operands must be numbers.",
                        )),
                    }
                }

                let left = self.visit_expr(&mut *left)?;

                match op.type_ {
                    TokenType::Or if left.value().is_truthy() => {
                        return Ok(left)
                    }
                    TokenType::Or if !left.value().is_truthy() => {
                        return self.visit_expr(&mut *right)
                    }
                    TokenType::And if !left.value().is_truthy() => {
                        return Ok(left)
                    }
                    TokenType::And if left.value().is_truthy() => {
                        return self.visit_expr(&mut *right)
                    }
                    _ => (),
                }

                let right = self.visit_expr(&mut *right)?;

                match op.type_ {
                    TokenType::Plus => match (left.value(), right.value()) {
                        (Value::Number(l), Value::Number(r)) => {
                            Ok(ValueRef::from_value(Value::Number(l + r)))
                        }
                        (Value::String(l), Value::String(r)) => {
                            Ok(ValueRef::from_value(Value::String(l + &r)))
                        }
                        _ => Err(RuntimeError::new(
                            Some(op),
                            "Operands must be two numbers or two strings.",
                        )),
                    },
                    TokenType::Minus => num_op(op, left, right, |l, r| {
                        ValueRef::from_value(Value::Number(l - r))
                    }),
                    TokenType::Star => num_op(op, left, right, |l, r| {
                        ValueRef::from_value(Value::Number(l * r))
                    }),
                    // TokenType::Slash if right == Value::Number(0.0) => Err(
                    //     RuntimeError::new(Some(op), "Can't divide by zero."),
                    // ),
                    TokenType::Slash => num_op(op, left, right, |l, r| {
                        ValueRef::from_value(Value::Number(l / r))
                    }),

                    TokenType::Greater => num_op(op, left, right, |l, r| {
                        ValueRef::from_value(Value::Bool(l > r))
                    }),
                    TokenType::GreaterEqual => {
                        num_op(op, left, right, |l, r| {
                            ValueRef::from_value(Value::Bool(l >= r))
                        })
                    }
                    TokenType::Less => num_op(op, left, right, |l, r| {
                        ValueRef::from_value(Value::Bool(l < r))
                    }),
                    TokenType::LessEqual => num_op(op, left, right, |l, r| {
                        ValueRef::from_value(Value::Bool(l <= r))
                    }),

                    TokenType::EqualEqual => {
                        Ok(ValueRef::from_value(Value::Bool(left == right)))
                    }
                    TokenType::BangEqual => {
                        Ok(ValueRef::from_value(Value::Bool(left != right)))
                    }
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
                let mut arguments: Vec<ValueRef> = arguments
                    .iter_mut()
                    .map(|e| self.visit_expr(e))
                    .collect::<Result<_, _>>()?;

                let wrong_arity = |e| {
                    Err(RuntimeError::new(
                        Some(right_paren),
                        format!(
                            "Expected {} arguments but got {}.",
                            e,
                            arguments.len()
                        ),
                    ))
                };
                match callee.value() {
                    Value::Fun(mut f) if f.arity() == arguments.len() => {
                        f.call(self, &mut arguments)
                    }
                    Value::Fun(f) => wrong_arity(f.arity()),
                    Value::Class(class) => {
                        let instance = ValueRef::from_value(Value::Instance(
                            Instance::new(class.clone()),
                        ));
                        match class.find_method("init") {
                            Some(init) if init.arity() == arguments.len() => {
                                init.bind(&instance)?
                                    .call(self, &mut arguments)?;
                                Ok(instance)
                            }
                            None if arguments.is_empty() => Ok(instance),
                            Some(init) => wrong_arity(init.arity()),
                            None => wrong_arity(0),
                        }
                    }
                    _ => Err(RuntimeError::new(
                        Some(right_paren),
                        "Can only call functions and classes.",
                    )),
                }
            }

            Expr::Get { object, name } => {
                let object = self.visit_expr(object)?;
                let value = &*object.get();
                if let Value::Instance(instance) = value {
                    instance.get(&object, name)
                } else {
                    Err(RuntimeError::new(
                        Some(name),
                        "Only instances have properties.",
                    ))
                }
            }

            Expr::Grouping { expr } => self.visit_expr(expr),

            Expr::Literal { value } => Ok(ValueRef::from_value(value.clone())),

            Expr::Set {
                object,
                name,
                value,
            } => {
                let object = self.visit_expr(object)?;
                let value = self.visit_expr(value)?;
                let get_mut = &mut *object.get_mut();
                if let Value::Instance(instance) = get_mut {
                    instance.set(name, value.clone());
                    Ok(value)
                } else {
                    Err(RuntimeError::new(
                        Some(name),
                        "Only instances have fields.",
                    ))
                }
            }

            Expr::This { keyword } => {
                self.lookup_variable(&keyword.clone(), expr)
            }

            Expr::Unary { op, right } => {
                let value = self.visit_expr(&mut *right)?;
                Ok(match op.type_ {
                    TokenType::Minus => {
                        let value =
                            value.value().as_number().ok_or_else(|| {
                                RuntimeError::new(
                                    Some(op),
                                    "Operand must be a number.",
                                )
                            })?;
                        ValueRef::from_value(Value::Number(-value))
                    }
                    TokenType::Bang => ValueRef::from_value(Value::Bool(
                        !value.value().is_truthy(),
                    )),
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
                self.current.define(name.lexeme.clone(), ValueRef::nil());

                let mut methods = BTreeMap::new();
                for method in stmt_methods {
                    let closure = self.current.clone();
                    let is_init = method.name.lexeme == "init";
                    let function =
                        LoxFunction::new(method.clone(), closure, is_init);
                    methods.insert(method.name.lexeme.clone(), function);
                }

                let class = Class::new(name.lexeme.clone(), methods);
                self.current.define(
                    name.lexeme.clone(),
                    ValueRef::from_value(Value::Class(class)),
                );
                Ok(())
            }

            Stmt::Expression { expr } => self.visit_expr(expr).map(drop),

            Stmt::Function(declaration) => {
                let closure = self.current.enclose();
                let function = ValueRef::from_value(Value::Fun(Fun::Lox(
                    LoxFunction::new(declaration.clone(), closure, false),
                )));
                self.current
                    .define(declaration.name.lexeme.clone(), function);
                Ok(())
            }

            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.visit_expr(condition)?.value().is_truthy() {
                    self.visit_stmt(then_branch)?;
                } else if let Some(else_branch) = else_branch {
                    self.visit_stmt(else_branch)?;
                }
                Ok(())
            }

            Stmt::PrintStmt { expr } => {
                let value = self.visit_expr(expr)?;
                writeln!(self.output, "{}", value.value())
                    .map_err(|e| RuntimeError::new(None, e.to_string()))
            }

            Stmt::Return { keyword: _, value } => Err(ControlFlow::Return(
                value
                    .as_mut()
                    .map(|e| self.visit_expr(e))
                    .transpose()?
                    .unwrap_or(ValueRef::nil()),
            )),

            Stmt::Var { name, init } => {
                let value = init
                    .as_mut()
                    .map(|e| self.visit_expr(e))
                    .transpose()?
                    .unwrap_or(ValueRef::nil());
                self.current.define(name.lexeme.clone(), value);
                Ok(())
            }

            Stmt::While { condition, body } => {
                while self.visit_expr(condition)?.value().is_truthy() {
                    self.visit_stmt(body)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests;
