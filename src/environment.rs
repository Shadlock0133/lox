use std::{
    collections::BTreeMap,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
    errors::{RuntimeError, RuntimeResult},
    tokens::Token,
    types::Value,
};

#[derive(Clone)]
pub struct Environment {
    inner: Arc<RwLock<Inner>>,
}

impl fmt::Debug for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.read().unwrap();
        f.debug_struct("Environment")
            .field("enclosing", &inner.enclosing)
            .field("values", &inner.values)
            .finish()
    }
}

impl Hash for Environment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.read().hash(state);
    }
}

#[derive(Default)]
struct Inner {
    enclosing: Option<Environment>,
    values: BTreeMap<String, Value>,
}

impl Hash for Inner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(en) = &self.enclosing {
            en.hash(state);
        }
        self.values.hash(state);
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
        self.inner.try_read().unwrap()
    }

    fn write(&mut self) -> RwLockWriteGuard<Inner> {
        self.inner.try_write().unwrap()
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.write().values.insert(name, value);
    }

    pub fn assign(
        &mut self,
        name: &Token,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let mut write = self.write();
        if let Some(v) = write.values.get_mut(&name.lexeme) {
            *v = value;
            Ok(())
        } else if let Some(ref mut en) = write.enclosing {
            en.assign(name, value)
        } else {
            Err(RuntimeError::new(
                Some(name),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    pub fn get(&self, name: &Token) -> RuntimeResult<Value> {
        if let Some(value) = self.read().values.get(&name.lexeme) {
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

    pub fn get_at(
        &self,
        distance: usize,
        name: &Token,
    ) -> RuntimeResult<Value> {
        self.ancestor(distance)
            .ok_or_else(|| {
                RuntimeError::new(Some(name), "Non-existent env ancestor")
            })?
            .read()
            .values
            .get(&name.lexeme)
            .ok_or_else(|| {
                RuntimeError::new(
                    Some(name),
                    format!("Missing variable at {} dist", distance),
                )
            })
            .map(Clone::clone)
    }

    fn ancestor(&self, distance: usize) -> Option<Environment> {
        let mut env = self.clone();
        for _ in 0..distance {
            let environment = env.read().enclosing.as_ref()?.clone();
            env = environment;
        }
        Some(env)
    }
}
