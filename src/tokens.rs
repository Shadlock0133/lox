use crate::{environment::Environment, interpreter::{Interpreter, RuntimeError}, syntax};
use std::{cell::RefCell, fmt, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    Identifier,
    String,
    Number,

    And,
    Break,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Eof,
}

#[derive(Clone)]
pub enum Fun {
    Foreign(Rc<dyn Fn(&mut Interpreter, &mut [Value]) -> Value>, usize),
    Native(Rc<RefCell<syntax::Function>>),
}

impl Fun {
    pub fn call(&mut self, interpreter: &mut Interpreter, arguments: &mut [Value]) -> Result<Value, RuntimeError> {
        match self {
            Self::Foreign(closure, _) => Ok((closure)(interpreter, arguments)),
            Self::Native(function) => {
                let mut function = function.borrow_mut();
                let environment = Environment::from_enclosing(&interpreter.global);
                for (param, arg) in function.params.iter().zip(arguments.iter()) {
                    environment.borrow_mut().define(param.lexeme.clone(), arg.clone());
                }
                interpreter.execute_block(&mut function.body, environment)?;
                Ok(Value::Nil)
            }
        }
    }

    pub fn arity(&self) -> usize {
        match self {
            Self::Foreign(_, arity) => *arity,
            Self::Native(function) => function.borrow().params.len(),
        }
    }
}

impl fmt::Debug for Fun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Foreign(_, _) => write!(f, "<foreign fn>"),
            Self::Native(function) => write!(f, "<fn {}>", function.borrow().name.lexeme),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Fun(Fun),
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

impl Value {
    pub fn fun<F: Fn(&mut Interpreter, &mut [Value]) -> Value + 'static>(
        arity: usize,
        f: F,
    ) -> Self {
        Value::Fun(Fun::Foreign(Rc::new(f), arity))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(l), Value::Number(r)) if l.is_nan() && r.is_nan() => true,
            (Value::Nil, Value::Nil) => true,
            (Value::Number(l), Value::Number(r)) => l == r,
            (Value::String(l), Value::String(r)) => l == r,
            (Value::Bool(l), Value::Bool(r)) => l == r,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Fun(fun) => write!(f, "{:?}", fun),
            Value::String(s) => write!(f, "{}", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
        }
    }
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub type_: TokenType,
    pub lexeme: String,
    pub literal: Option<Value>,
    pub line: u32,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {}", self.type_, self.lexeme)?;
        if let Some(literal) = &self.literal {
            write!(f, " {:?}", literal)?;
        }
        Ok(())
    }
}
