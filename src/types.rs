use std::{fmt, sync::Arc};

use crate::{
    // environment::Environment,
    errors::RuntimeError,
    interpreter::Interpreter,
    tokens::Token,
};

// #[derive(Debug, Clone, Hash, Eq)]
#[derive(Debug, Clone)]
pub enum Value {
    // Fun(Fun),
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

impl Value {
    // pub fn fun<F: Fn(&mut Interpreter, &mut [Value]) -> Value + 'static>(
    //     arity: usize,
    //     f: F,
    // ) -> Self {
    //     Value::Fun(Fun::Foreign {
    //         inner: Rc::new(f),
    //         arity,
    //     })
    // }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Number(l), Self::Number(r)) if l.is_nan() && r.is_nan() => true,
            (Self::Number(l), Self::Number(r)) => l == r,
            (Self::String(l), Self::String(r)) => l == r,
            (Self::Bool(l), Self::Bool(r)) => l == r,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Self::Fun(fun) => write!(f, "{:?}", fun),
            Self::String(s) => write!(f, "{}", s),
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

// #[derive(Clone)]
// pub enum Fun {
//     Foreign {
//         inner: Arc<dyn Fn(&mut Interpreter, &mut [Value]) -> Value>,
//         arity: usize,
//     },
//     Native {
//         name: Box<Token>,
//         params: Vec<Token>,
//         body: Vec<crate::syntax::Stmt>,
//         closure: Environment,
//     },
// }

// impl Fun {
//     pub fn call(
//         &mut self,
//         interpreter: &mut Interpreter,
//         arguments: &mut [Value],
//     ) -> Result<Value, RuntimeError> {
//         match self {
//             Self::Foreign { inner, .. } => Ok((inner)(interpreter, arguments)),
//             Self::Native {
//                 params,
//                 body,
//                 closure,
//                 ..
//             } => {
//                 let environment = closure.enclose();
//                 for (param, arg) in params.iter().zip(arguments.iter()) {
//                     environment
//                         .define(param.lexeme.clone(), arg.clone());
//                 }
//                 let result = interpreter.execute_block(body, environment);
//                 match result {
//                     Ok(()) => Ok(Value::Nil),
//                     Err(RuntimeError::Return(value)) => Ok(value),
//                     Err(err) => Err(err),
//                 }
//             }
//         }
//     }

//     pub fn arity(&self) -> usize {
//         match self {
//             Self::Foreign { arity, .. } => *arity,
//             Self::Native { params, .. } => params.len(),
//         }
//     }
// }

// impl fmt::Debug for Fun {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             Self::Foreign { .. } => write!(f, "<foreign fn>"),
//             Self::Native { name, .. } => write!(f, "<fn {}>", name.lexeme),
//         }
//     }
// }
