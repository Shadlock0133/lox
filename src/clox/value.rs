use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(Number),
}

impl Value {
    pub fn number(value: f64) -> Self {
        Self::Number(Number(value))
    }

    pub fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    pub fn nil() -> Self {
        Self::Nil
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
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Number(pub f64);

impl fmt::Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.abs().log10() <= 6.0 {
            write!(f, "{}", self.0)
        } else {
            write!(f, "{:e}", self.0)
        }
    }
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
