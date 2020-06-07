use crate::{
    environment::Environment,
    errors::{RuntimeError, RuntimeResult},
    impl_visitor,
    syntax::*,
    tokens::{Fun, Token, TokenType, Value},
    visitor::*,
};
use std::{cell::RefCell, io::Write, rc::Rc, time::Instant};

pub struct Interpreter {
    start_time: Instant,
    output: Box<dyn Write>,
    pub global: Rc<RefCell<Environment>>,
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

    pub fn interpret(&mut self, statements: &mut [Stmt]) -> RuntimeResult<()> {
        let result = (|| {
            for statement in statements {
                self.visit(statement)?;
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
        environment: Rc<RefCell<Environment>>,
    ) -> RuntimeResult<()> {
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

impl ExprVisitor<RuntimeResult<Value>> for Interpreter {
    fn visit_assign(&mut self, t: &mut Assign) -> RuntimeResult<Value> {
        let value = self.visit(&mut *t.value)?;
        self.current
            .borrow_mut()
            .assign(&t.name, value.clone())?;
        Ok(value)
    }
}
impl StmtVisitor<RuntimeResult> for Interpreter {}

impl_visitor! { for Interpreter, (&mut self, t: Assign) -> RuntimeResult<Value> {
    let value = self.visit(&mut *t.value)?;
    self.current
        .borrow_mut()
        .assign(&t.name, value.clone())?;
    Ok(value)
}}

impl_visitor! { for Interpreter, (&mut self, t: Binary) -> RuntimeResult<Value> {
    fn num_op<F: Fn(f64, f64) -> Value>(
        op: &Token,
        l: Value,
        r: Value,
        f: F,
    ) -> RuntimeResult<Value> {
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
                "Operands must begin with a string or be two numbers.",
            )),
        },
        TokenType::Minus => num_op(&t.op, left, right, |l, r| Value::Number(l - r)),
        TokenType::Star => num_op(&t.op, left, right, |l, r| Value::Number(l * r)),
        TokenType::Slash if right == Value::Number(0.0) => {
            Err(RuntimeError::new(&t.op, "Can't divide by zero."))
        }
        TokenType::Slash => num_op(&t.op, left, right, |l, r| Value::Number(l / r)),

        TokenType::Greater => num_op(&t.op, left, right, |l, r| Value::Bool(l > r)),
        TokenType::GreaterEqual => num_op(&t.op, left, right, |l, r| Value::Bool(l >= r)),
        TokenType::Less => num_op(&t.op, left, right, |l, r| Value::Bool(l < r)),
        TokenType::LessEqual => num_op(&t.op, left, right, |l, r| Value::Bool(l <= r)),

        TokenType::EqualEqual => Ok(Value::Bool(left == right)),
        TokenType::BangEqual => Ok(Value::Bool(left != right)),
        _ => Err(RuntimeError::new(&t.op, "Invalid binary operator.")),
    }
}}

impl_visitor! { for Interpreter, (&mut self, t: Call) -> RuntimeResult<Value> {
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
            Ok(fun.call(self, &mut arguments)?)
        },
        _ => Err(RuntimeError::new(&t.right_paren, "Can only call functions and classes.")),
    }
}}

impl_visitor! { for Interpreter, (&mut self, t: Grouping) -> RuntimeResult<Value> {
    self.visit(&mut *t.expr)
}}

impl_visitor! { for Interpreter, (&mut self, t: Literal) -> RuntimeResult<Value> {
    Ok(t.value.clone())
}}

impl_visitor! { for Interpreter, (&mut self, t: Unary) -> RuntimeResult<Value> {
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
            "Unary expression must contain '-' or '!'.",
        )),
    }
}}

impl_visitor! { for Interpreter, (&mut self, t: Variable) -> RuntimeResult<Value> {
    self.current.borrow().get(&t.name)
}}

impl_visitor! { for Interpreter, (&mut self, t: Block) -> RuntimeResult<()> {
    self.execute_block(
        &mut t.statements,
        Environment::from_enclosing(&self.current),
    )
}}

impl_visitor! { for Interpreter, (&mut self, t: Expression) -> RuntimeResult<()> {
    self.visit(&mut t.expr)?;
    Ok(())
}}

impl_visitor! { for Interpreter, (&mut self, t: Function) -> RuntimeResult<()> {
    let env_name = t.name.lexeme.clone();
    let name = Box::new(t.name.clone());
    let params = t.params.clone();
    let body = t.body.clone();
    let closure = Environment::from_enclosing(&self.current);
    let function = Value::Fun(Fun::Native { name, params, body, closure });
    self.current.borrow_mut().define(env_name, function);
    Ok(())
}}

impl_visitor! { for Interpreter, (&mut self, t: If) -> RuntimeResult<()> {
    if self.visit(&mut t.condition)?.is_truthy() {
        self.visit(&mut *t.then_branch)?;
    } else if let Some(else_branch) = &mut t.else_branch {
        self.visit(&mut **else_branch)?;
    }
    Ok(())
}}

impl_visitor! { for Interpreter, (&mut self, t: PrintStmt) -> RuntimeResult<()> {
    let value = self.visit(&mut t.expr)?;
    let _ = writeln!(self.output, "{}", value);
    Ok(())
}}

impl_visitor! { for Interpreter, (&mut self, t: Return) -> RuntimeResult<()> {
    let value = t.value
        .as_mut()
        .map(|x| self.visit(x))
        .transpose()?
        .unwrap_or(Value::Nil);
    Err(RuntimeError::Return(value))
}}

impl_visitor! { for Interpreter, (&mut self, t: Var) -> RuntimeResult<()> {
    let value = match &mut t.init {
        Some(expr) => self.visit(expr)?,
        None => Value::Nil,
    };
    self.current
        .borrow_mut()
        .define(t.name.lexeme.clone(), value);
    Ok(())
}}

impl_visitor! { for Interpreter, (&mut self, t: While) -> RuntimeResult<()> {
    while self.visit(&mut t.condition)?.is_truthy() {
        self.visit(&mut *t.body)?;
    }
    Ok(())
}}
