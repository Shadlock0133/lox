use crate::{tokens::{Value, Token}};
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum RuntimeError {
    Return(Value),
    Break,
    Error(Token, String),
}

pub type RuntimeResult<T = ()> = Result<T, RuntimeError>;

impl RuntimeError {
    pub fn new<S: Into<String>>(token: &Token, message: S) -> Self {
        Self::Error(token.clone(), message.into())
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Error(token, message) => write!(f, "{}\n[line {}]", message, token.line),
            _ => unimplemented!(),
        }
    }
}

impl Error for RuntimeError {}

#[derive(Debug)]
pub struct ParseError;

pub type ParseResult<T = ()> = Result<T, ParseError>;

#[derive(Debug)]
pub struct ResolveError;

pub type ResolveResult<T = ()> = Result<T, ResolveError>;
