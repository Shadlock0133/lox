use crate::{interpreter::RuntimeError, tokens::Token, tokens::Value};
use std::collections::HashMap;

pub struct Environment {
    values: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
        self.values
            .get(&name.lexeme)
            .map(Clone::clone)
            .ok_or_else(|| {
                RuntimeError::new(&name, format!("Undefined variable '{}'.", name.lexeme))
            })
    }
}
