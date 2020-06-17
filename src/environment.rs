use std::{cell::{RefCell, Ref, RefMut}, collections::HashMap, rc::Rc};

use crate::{errors::RuntimeError, tokens::Token, types::Value};

#[derive(Clone, Hash)]
pub struct Environment {
    inner: Rc<RefCell<Inner>>,
}

#[derive(Default)]
struct Inner {
    enclosing: Option<Environment>,
    values: HashMap<String, Value>,
}

impl Inner {
    fn new(enclosing: Environment) -> Self {
        Self {
            enclosing: Some(enclosing), ..Default::default()
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }

    pub fn enclose(&self) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner::new(self.clone())))
        }
    }

    fn borrow(&self) -> Ref<Inner> {
        self.inner.borrow()
    }

    fn borrow_mut(&self) -> RefMut<Inner> {
        self.inner.borrow_mut()
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.inner.borrow_mut().values.insert(name, value);
    }

    pub fn assign(&mut self, name: &Token, value: Value) -> Result<(), RuntimeError> {
        if let Some(v) = self.inner.borrow_mut().values.get_mut(&name.lexeme) {
            *v = value;
            Ok(())
        } else if let Some(en) = &mut self.enclosing {
            en.borrow_mut().assign(name, value)
        } else {
            Err(RuntimeError::new(
                &name,
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    pub fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.values.get(&name.lexeme) {
            Ok(value.clone())
        } else if let Some(en) = &self.enclosing {
            en.borrow().get(name)
        } else {
            Err(RuntimeError::new(
                &name,
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }
}
