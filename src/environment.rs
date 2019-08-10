use crate::{interpreter::RuntimeError, tokens::Token, tokens::Value};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub struct Environment {
    enclosing: Option<Rc<RefCell<Self>>>,
    values: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            enclosing: None,
            values: HashMap::new(),
        }))
    }

    pub fn from_enclosing(old: &Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            enclosing: Some(Rc::clone(old)),
            values: HashMap::new(),
        }))
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn assign(&mut self, name: &Token, value: Value) -> Result<(), RuntimeError> {
        if let Some(v) = self.values.get_mut(&name.lexeme) {
            *v = value;
            Ok(())
        } else if let Some(en) = &mut self.enclosing {
            en.borrow_mut().assign(name, value)
        } else {
            Err(RuntimeError::new(&name, format!("Undefined variable '{}'.", name.lexeme)))
        }
    }

    pub fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.values.get(&name.lexeme) {
            Ok(value.clone())
        } else if let Some(en) = &self.enclosing {
            en.borrow().get(name)
        } else {
            Err(RuntimeError::new(&name, format!("Undefined variable '{}'.", name.lexeme)))
        }
    }
}
