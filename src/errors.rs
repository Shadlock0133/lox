use crate::{tokens::Token, types::Value};

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Unexpected return")]
    Return(Value),
    #[error("Unexpected break")]
    Break,
    #[error("Runtime error at {0}: {1}")]
    Error(Token, String),
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

impl RuntimeError {
    pub fn new<S: Into<String>>(token: &Token, message: S) -> Self {
        Self::Error(token.clone(), message.into())
    }
}

#[derive(Debug)]
pub struct ParseError;

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug)]
pub struct ResolveError;

pub type ResolveResult<T> = Result<T, ResolveError>;
