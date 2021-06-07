use std::fmt;

#[derive(Clone)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    Obj(Box<Obj>),
}

impl Value {
    pub fn number(value: f64) -> Self {
        Self::Number(value)
    }

    pub fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    pub fn nil() -> Self {
        Self::Nil
    }

    pub fn string(value: String) -> Self {
        Self::Obj(Box::new(Obj::String(value)))
    }

    pub fn is_falsey(&self) -> bool {
        matches!(self, Self::Nil | Self::Bool(false))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Obj(a), Value::Obj(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => {
                if n.abs().log2() <= 20.0 {
                    write!(f, "{}", n)
                } else {
                    write!(f, "{:e}", n)
                }
            }
            Value::Obj(o) => match o.as_ref() {
                Obj::String(s) => write!(f, "{}", s),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    String(String),
}

#[derive(Default)]
pub struct ValueArray {
    pub(super) values: Vec<Value>,
}

impl ValueArray {
    pub fn write(&mut self, value: Value) {
        self.values.push(value)
    }
}
