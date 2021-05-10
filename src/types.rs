use std::{
    collections::BTreeMap,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
    environment::Environment,
    errors::{ControlFlow, RuntimeError, RuntimeResult},
    interpreter::Interpreter,
    tokens::Token,
};

#[derive(Debug, Clone)]
pub struct ValueRef(Arc<RwLock<Value>>);

impl PartialEq for ValueRef {
    fn eq(&self, other: &Self) -> bool {
        match (self.value(), other.value()) {
            (Value::Instance(_), Value::Instance(_)) => {
                Arc::ptr_eq(&self.0, &other.0)
            }
            _ => self.0.read().unwrap().eq(&*other.0.read().unwrap()),
        }
    }
}

impl Eq for ValueRef {}

impl Hash for ValueRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.read().unwrap().hash(state)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Class(Class),
    Instance(Instance),
    Fun(Fun),
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

impl ValueRef {
    pub fn fun<F>(arity: usize, f: F) -> Self
    where
        F: Fn(&mut Interpreter, &mut [ValueRef]) -> RuntimeResult<ValueRef>
            + Send
            + Sync
            + 'static,
    {
        Self::from_value(Value::Fun(Fun::Native {
            inner: Arc::new(f),
            arity,
        }))
    }

    pub fn from_value(value: Value) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn value(&self) -> Value {
        self.0.read().unwrap().clone()
    }

    pub fn get(&self) -> RwLockReadGuard<Value> {
        self.0.read().unwrap()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<Value> {
        self.0.write().unwrap()
    }

    pub fn is_instance(&self) -> bool {
        matches!(self.value(), Value::Instance(_))
    }

    pub fn nil() -> Self {
        Self::from_value(Value::Nil)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Class(l), Self::Class(r)) => l == r,
            (Self::Nil, Self::Nil) => true,
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
            Self::Instance(i) => i.hash(state),
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
            Self::Instance(i) => write!(f, "{}", i),
            Self::Fun(fun) => write!(f, "{:?}", fun),
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) if n.is_sign_negative() && *n == 0.0 => {
                write!(f, "-0")
            }
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

type NativeFun = Arc<
    dyn (Fn(&mut Interpreter, &mut [ValueRef]) -> RuntimeResult<ValueRef>)
        + Send
        + Sync,
>;

#[derive(Clone)]
pub enum Fun {
    Native { inner: NativeFun, arity: usize },
    Lox(LoxFunction),
}

impl Fun {
    pub fn call(
        &mut self,
        interpreter: &mut Interpreter,
        arguments: &mut [ValueRef],
    ) -> RuntimeResult<ValueRef> {
        match self {
            Self::Native { inner, .. } => (inner)(interpreter, arguments),
            Self::Lox(LoxFunction {
                params,
                body,
                closure,
                ..
            }) => {
                let mut environment = closure.enclose();
                for (param, arg) in params.iter().zip(arguments.iter()) {
                    environment.define(param.lexeme.to_string(), arg.clone());
                }
                let result = interpreter.execute_block(body, environment);
                match result {
                    Ok(()) => Ok(ValueRef::nil()),
                    Err(ControlFlow::Return(value)) => Ok(value),
                    Err(err) => Err(err),
                }
            }
        }
    }

    pub fn arity(&self) -> usize {
        match self {
            Self::Native { arity, .. } => *arity,
            Self::Lox(LoxFunction { params, .. }) => params.len(),
        }
    }
}

impl fmt::Debug for Fun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Native { .. } => write!(f, "<native fn>"),
            Self::Lox(LoxFunction { name, .. }) => {
                write!(f, "<fn {}>", name.lexeme)
            }
        }
    }
}

impl Hash for Fun {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Native { inner, arity } => {
                Arc::as_ptr(&inner).hash(state);
                arity.hash(state);
            }
            Self::Lox { .. } => (),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LoxFunction {
    pub name: Box<Token>,
    pub params: Vec<Token>,
    pub body: Vec<crate::ast::Stmt>,
    pub closure: Environment,
}

impl LoxFunction {
    fn bind(&self, instance: &ValueRef) -> RuntimeResult<Self> {
        if !instance.is_instance() {
            return Err(RuntimeError::new(
                Some(&*self.name),
                "Trying to bind method without instance",
            ));
        }
        let mut closure = self.closure.enclose();
        closure.define("this".into(), instance.clone());
        Ok(Self {
            closure,
            ..self.clone()
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Class {
    name: String,
    methods: BTreeMap<String, LoxFunction>,
}

impl Class {
    pub fn new(name: String, methods: BTreeMap<String, LoxFunction>) -> Self {
        Self { name, methods }
    }

    fn find_method(&self, name: &Token) -> Option<&LoxFunction> {
        self.methods.get(&name.lexeme)
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Hash)]
pub struct Instance {
    class: Class,
    fields: BTreeMap<String, ValueRef>,
}

impl Instance {
    pub fn new(class: Class) -> Self {
        Self {
            class,
            fields: BTreeMap::new(),
        }
    }

    pub fn get(
        &self,
        true_self: &ValueRef,
        name: &Token,
    ) -> RuntimeResult<ValueRef> {
        if let Some(field) = self.fields.get(&name.lexeme) {
            return Ok(field.clone());
        }

        if let Some(method) = self.class.find_method(&name) {
            return Ok(ValueRef::from_value(Value::Fun(Fun::Lox(
                method.bind(true_self)?,
            ))));
        }

        Err(RuntimeError::new(
            Some(name),
            format!("Undefined property '{}'.", name.lexeme),
        ))
    }

    pub fn set(&mut self, name: &Token, value: ValueRef) {
        self.fields.insert(name.lexeme.clone(), value);
    }
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} instance", self.class.name)
    }
}
