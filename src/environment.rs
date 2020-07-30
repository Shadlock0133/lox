use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::{errors::RuntimeError, tokens::Token, types::Value};

#[derive(Clone)]
pub struct Environment {
    inner: Arc<RwLock<Inner>>,
}

impl Hash for Environment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.read().hash(state);
    }
}

#[derive(Default)]
struct Inner {
    enclosing: Option<Environment>,
    values: HashMap<String, Value>,
}

impl Hash for Inner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(en) = &self.enclosing {
            en.hash(state);
        }
        let mut map = self.values.iter().collect::<Vec<_>>();
        map.sort_by_key(|x| x.0);
        map.hash(state);
    }
}

impl Inner {
    fn new(enclosing: Environment) -> Self {
        Self {
            enclosing: Some(enclosing),
            ..Default::default()
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn enclose(&self) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::new(self.clone()))),
        }
    }

    fn read(&self) -> RwLockReadGuard<Inner> {
        self.inner.read().unwrap()
    }

    fn write(&self) -> RwLockWriteGuard<Inner> {
        self.inner.write().unwrap()
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.write().values.insert(name, value);
    }

    pub fn assign(&mut self, name: &Token, value: Value) -> Result<(), RuntimeError> {
        if let Some(v) = self.write().values.get_mut(&name.lexeme) {
            *v = value;
            Ok(())
        } else if let Some(en) = &mut self.write().enclosing {
            en.assign(name, value)
        } else {
            Err(RuntimeError::new(
                Some(name),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    pub fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.write().values.get(&name.lexeme) {
            Ok(value.clone())
        } else if let Some(en) = &self.read().enclosing {
            en.get(name)
        } else {
            Err(RuntimeError::new(
                Some(name),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }
}
