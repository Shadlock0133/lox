use std::fmt;

#[derive(Clone, Copy, PartialEq)]
pub struct Value(pub f64);

impl fmt::Debug for Value {
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
