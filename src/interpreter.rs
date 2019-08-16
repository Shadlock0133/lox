use crate::{
    environment::Environment,
    impl_visitor,
    syntax::*,
    tokens::{Token, TokenType, Value},
    visitor::*,
};
use std::{cell::RefCell, fmt, io::Write, rc::Rc, time::Instant};

pub struct Interpreter {
    start_time: Instant,
    output: Box<dyn Write>,
    #[allow(dead_code)]
    global: Rc<RefCell<Environment>>,
    current: Rc<RefCell<Environment>>,
}

impl Interpreter {
    pub fn new<W: Write + 'static>(output: W) -> Self {
        let global = Environment::new();

        global.borrow_mut().define(
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

    pub fn interpret(&mut self, statements: &mut [Stmt]) -> Result<(), RuntimeError> {
        for statement in statements {
            self.visit(statement)?;
        }
        Ok(())
    }

    fn execute_block(
        &mut self,
        statements: &mut [Stmt],
        environment: Rc<RefCell<Environment>>,
    ) -> Result<(), RuntimeError> {
        let previous = Rc::clone(&self.current);
        let result = (|| {
            self.current = environment;
            for statement in statements {
                self.visit(statement)?;
            }
            Ok(())
        })();
        self.current = previous;
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

impl_visitor! { for Interpreter, (self, t: Assign) -> Result<Value, RuntimeError> {
    let value = self.visit(&mut *t.value)?;
    self.current
        .borrow_mut()
        .assign(&t.name, value.clone())?;
    Ok(value)
}}

impl_visitor! { for Interpreter, (self, t: Binary) -> Result<Value, RuntimeError> {
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
            (Value::String(l), r) => Ok(Value::String(l + &r.to_string())),
            (l, Value::String(r)) => Ok(Value::String(l.to_string() + &r)),
            _ => Err(RuntimeError::new(
                &t.op,
                "Operands must begin with a string or be two numbers",
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
        _ => Err(RuntimeError::new(&t.op, "Invalid binary operator")),
    }
}}

impl_visitor! { for Interpreter, (self, t: Call) -> Result<Value, RuntimeError> {
    let callee = self.visit(&mut *t.callee)?;

    let mut arguments = t.arguments
        .iter_mut()
        .map(|arg| self.visit(arg))
        .collect::<Result<Vec<_>, _>>()?;

    match callee {
        Value::Fun(mut fun) => {
            if arguments.len() != fun.arity() {
                return Err(RuntimeError::new(
                    &t.right_paren,
                    format!("Expected {} arguments but got {}.", fun.arity(), arguments.len())
                ));
            }
            Ok(fun.call(self, &mut arguments))
        },
        _ => Err(RuntimeError::new(&t.right_paren, "Can only call functions and classes.")),
    }
}}

impl_visitor! { for Interpreter, (self, t: Grouping) -> Result<Value, RuntimeError> {
    self.visit(&mut *t.expr)
}}

impl_visitor! { for Interpreter, (self, t: Literal) -> Result<Value, RuntimeError> {
    Ok(t.value.clone())
}}

impl_visitor! { for Interpreter, (self, t: Unary) -> Result<Value, RuntimeError> {
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
}}

impl_visitor! { for Interpreter, (self, t: Variable) -> Result<Value, RuntimeError> {
    self.current.borrow().get(&t.name)
}}

impl_visitor! { for Interpreter, (self, t: Block) -> Result<(), RuntimeError> {
    self.execute_block(
        &mut t.statements,
        Environment::from_enclosing(&self.current),
    )
}}

impl_visitor! { for Interpreter, (self, t: Expression) -> Result<(), RuntimeError> {
    self.visit(&mut t.expr)?;
    Ok(())
}}

impl_visitor! { for Interpreter, (self, t: If) -> Result<(), RuntimeError> {
    if self.visit(&mut t.condition)?.is_truthy() {
        self.visit(&mut *t.then_branch)?;
    } else if let Some(else_branch) = &mut t.else_branch {
        self.visit(&mut **else_branch)?;
    }
    Ok(())
}}

impl_visitor! { for Interpreter, (self, t: PrintStmt) -> Result<(), RuntimeError> {
    let value = self.visit(&mut t.expr)?;
    let _ = writeln!(self.output, "{}", value);
    Ok(())
}}

impl_visitor! { for Interpreter, (self, t: Var) -> Result<(), RuntimeError> {
    let value = match &mut t.init {
        Some(expr) => self.visit(expr)?,
        None => Value::Nil,
    };
    self.current
        .borrow_mut()
        .define(t.name.lexeme.clone(), value);
    Ok(())
}}

impl_visitor! { for Interpreter, (self, t: While) -> Result<(), RuntimeError> {
    while self.visit(&mut t.condition)?.is_truthy() {
        self.visit(&mut *t.body)?;
    }
    Ok(())
}}
