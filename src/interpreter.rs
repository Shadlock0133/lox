use crate::{
    ast::*,
    environment::Environment,
    errors::{RuntimeError, RuntimeResult},
    tokens::{Token, TokenType},
    types::Value,
};
use core::fmt;
use std::{collections::HashMap, io::Write, time::Instant};

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
                            "Operands must be a numbers.",
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
                            "Expected {} arguments but got {}",
                            f.arity(),
                            arguments.len()
                        ),
                    )),
                    _ => Err(RuntimeError::new(
                        Some(right_paren),
                        "Can only call functions and classes",
                    )),
                }
            }

            Expr::Grouping { expr } => self.visit_expr(expr),

            Expr::Literal { value } => Ok(value.clone()),

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
                    _ => Err(RuntimeError::new(
                        Some(op),
                        "Unary expression must contain '-' or '!'.",
                    ))?,
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

            Stmt::Expression { expr } => self.visit_expr(expr).map(drop),

            Stmt::Function { name, params, body } => {
                let closure = self.current.clone();
                let function = Value::Fun(crate::types::Fun::Native {
                    name: Box::new(name.clone()),
                    body: body.clone(),
                    params: params.clone(),
                    closure,
                });
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
mod tests {
    use crate::errors::ResolveError;

    use super::*;

    #[track_caller]
    fn run(x: &str) -> String {
        let tokens = crate::tokenizer::Tokenizer::new(x)
            .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut ast = crate::parser::Parser::new(tokens).parse().unwrap();

        let mut output = vec![];
        let mut interpreter = Interpreter::new(&mut output);

        crate::resolver::Resolver::new(&mut interpreter.locals)
            .resolve(&mut ast)
            .unwrap();

        interpreter.interpret(&mut ast).unwrap();
        drop(interpreter);
        String::from_utf8(output).unwrap()
    }

    #[track_caller]
    fn interpreter_error(x: &str) -> RuntimeError {
        let tokens = crate::tokenizer::Tokenizer::new(x)
            .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut ast = crate::parser::Parser::new(tokens).parse().unwrap();

        let mut output = vec![];
        let mut interpreter = Interpreter::new(&mut output);
        crate::resolver::Resolver::new(&mut interpreter.locals)
            .resolve(&mut ast)
            .unwrap();

        let error = interpreter.interpret(&mut ast).unwrap_err();
        error
    }

    #[track_caller]
    fn resolver_error(x: &str) -> ResolveError {
        let tokens = crate::tokenizer::Tokenizer::new(x)
            .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut ast = crate::parser::Parser::new(tokens).parse().unwrap();

        let mut output = vec![];
        let mut interpreter = Interpreter::new(&mut output);
        crate::resolver::Resolver::new(&mut interpreter.locals)
            .resolve(&mut ast)
            .unwrap_err()
    }

    #[test]
    fn simple() {
        assert_eq!(run("print \"one\";"), "one\n");
        assert_eq!(run("print true;"), "true\n");
        assert_eq!(run("print 1 + 2;"), "3\n");
        assert_eq!(run("var a = 1; print a;"), "1\n");
        assert_eq!(run("var a = 1; print a = 2;"), "2\n");
    }

    #[test]
    fn scopes() {
        assert_eq!(
            run("var a = 1;
                var b = 1;
                print a;
                print b;
                {
                    var a = 2;
                    b = 2;
                    print a;
                    print b;
                }
                print a;
                print b;"),
            "1\n1\n2\n2\n1\n2\n"
        );

        assert_eq!(
            run("var a = 1;
                {
                    fun print_a() {
                        print a;
                    }
                    print_a(); // 1

                    a = 2;
                    print_a(); // 2

                    var a = 3;
                    print_a(); // 2, not 3
                }"),
            "1\n2\n2\n"
        );
    }

    #[test]
    fn scope_error() {
        assert_eq!(
            resolver_error(
                "{
                    var a = 1;
                    var a = 2;
                }"
            )
            .to_string(),
            "[line 3] Resolve Error at a: Variable already exists"
        );

        assert_eq!(
            resolver_error(
                "var a = 1;
                {
                    var a = a + 2;
                    print a;
                }"
            )
            .to_string(),
            "[line 3] Resolve Error at a: Can't read local variable in its own initializer"
        );
    }

    #[test]
    fn factorial() {
        assert_eq!(
            run("fun fact(a) {
                    if (a <= 1)
                        return 1;
                    else
                        return a * fact(a - 1);
                }
                print fact(20);"),
            ((1..=20).map(|x| x as f64).product::<f64>().to_string() + "\n")
        );
    }
}
