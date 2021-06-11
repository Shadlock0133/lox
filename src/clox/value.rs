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
        Self::Obj(Box::new(Obj::ObjString(ObjString::new(value))))
    }

    pub fn is_falsey(&self) -> bool {
        matches!(self, Self::Nil | Self::Bool(false))
    }

    pub fn into_obj_string(self) -> Option<ObjString> {
        match self {
            Value::Obj(o) => match *o {
                Obj::ObjString(s) => Some(s),
            },
            _ => None,
        }
    }

    pub fn into_string(self) -> Option<String> {
        self.into_obj_string().map(|s| s.0.into_string())
    }
}

fn fnv_1a(bytes: &[u8]) -> u32 {
    let mut hash = 0x811C_9DC5;
    for &byte in bytes {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x100_0193);
    }
    hash
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
            Value::Bool(b) => write!(f, "{:?}", b),
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => write!(f, "{:?}", n),
            Value::Obj(o) => match o.as_ref() {
                Obj::ObjString(ObjString(s, _)) => write!(f, "{:?}", s),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjString(pub Box<str>, pub u32);

impl ObjString {
    pub fn new(value: String) -> Self {
        let hash = fnv_1a(value.as_bytes());
        Self(value.into_boxed_str(), hash)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    ObjString(ObjString),
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
