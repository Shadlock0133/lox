use std::{
    collections::BTreeMap,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
};

use crate::{
    environment::Environment,
    errors::{RuntimeError, RuntimeResult},
    interpreter::Interpreter,
    tokens::Token,
};

#[derive(Debug, Clone)]
pub enum Value {
    Class(Class),
    Instance(Arc<RwLock<Instance>>),
    Fun(Fun),
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

impl Value {
    pub fn fun<F>(arity: usize, f: F) -> Self
    where
        F: Fn(&mut Interpreter, &mut [Value]) -> Result<Value, RuntimeError>
            + Send
            + Sync
            + 'static,
    {
        Value::Fun(Fun::Foreign {
            inner: Arc::new(f),
            arity,
        })
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Class(l), Self::Class(r)) => l == r,
            (Self::Instance(l), Self::Instance(r)) => Arc::ptr_eq(l, r),
            (Self::Nil, Self::Nil) => true,
            (Self::Number(l), Self::Number(r)) if l.is_nan() && r.is_nan() => {
                true
            }
            (Self::Number(l), Self::Number(r)) => l == r,
            (Self::String(l), Self::String(r)) => l == r,
            (Self::Bool(l), Self::Bool(r)) => l == r,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Class(c) => c.hash(state),
            Self::Instance(i) => i.read().unwrap().hash(state),
            Self::Fun(f) => f.hash(state),
            Self::Number(n) => n.to_le_bytes().hash(state),
            Self::String(s) => s.hash(state),
            Self::Bool(b) => b.hash(state),
            Self::Nil => ().hash(state),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Class(c) => write!(f, "{}", c),
            Self::Instance(i) => write!(f, "{}", i.read().unwrap()),
            Self::Fun(fun) => write!(f, "{:?}", fun),
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => write!(f, "{}", n),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Nil => write!(f, "nil"),
        }
    }
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Nil => false,
            _ => true,
        }
    }
}

#[derive(Clone)]
pub enum Fun {
    Foreign {
        inner: Arc<
            dyn (Fn(
                    &mut Interpreter,
                    &mut [Value],
                ) -> Result<Value, RuntimeError>)
                + Send
                + Sync,
        >,
        arity: usize,
    },
    Native {
        name: Box<Token>,
        params: Vec<Token>,
        body: Vec<crate::ast::Stmt>,
        closure: Environment,
    },
}

impl Fun {
    pub fn call(
        &mut self,
        interpreter: &mut Interpreter,
        arguments: &mut [Value],
    ) -> Result<Value, RuntimeError> {
        match self {
            Self::Foreign { inner, .. } => (inner)(interpreter, arguments),
            Self::Native {
                params,
                body,
                closure,
                ..
            } => {
                let mut environment = closure.enclose();
                for (param, arg) in params.iter().zip(arguments.iter()) {
                    environment.define(param.lexeme.to_string(), arg.clone());
                }
                let result = interpreter.execute_block(body, environment);
                match result {
                    Ok(()) => Ok(Value::Nil),
                    Err(RuntimeError::Return(value)) => Ok(value),
                    Err(err) => Err(err),
                }
            }
        }
    }

    pub fn arity(&self) -> usize {
        match self {
            Self::Foreign { arity, .. } => *arity,
            Self::Native { params, .. } => params.len(),
        }
    }
}

impl fmt::Debug for Fun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Foreign { .. } => write!(f, "<foreign fn>"),
            Self::Native { name, .. } => write!(f, "<fn {}>", name.lexeme),
        }
    }
}

impl Hash for Fun {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Foreign { inner, arity } => {
                Arc::as_ptr(&inner).hash(state);
                arity.hash(state);
            }
            Self::Native { .. } => (),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Class {
    name: String,
}

impl Class {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Instance {
    class: Class,
    fields: BTreeMap<String, Value>,
}

impl Instance {
    pub fn new(class: Class) -> Self {
        Self {
            class,
            fields: BTreeMap::new(),
        }
    }

    pub fn get(&self, name: &Token) -> RuntimeResult<Value> {
        self.fields.get(&name.lexeme).cloned().ok_or_else(|| {
            RuntimeError::new(
                Some(name),
                format!("Undefined property {}", name.lexeme),
            )
        })
    }

    pub fn set(&mut self, name: &Token, value: Value) {
        self.fields.insert(name.lexeme.clone(), value);
    }
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} instance", self.class.name)
    }
}
