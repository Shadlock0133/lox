use std::fmt;

use crate::{
    tokens::{Token, TokenType},
    types::Value,
};

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

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("Unexpected character: {0}")]
    UnexpectedChar(char),
    #[error("Unterminated string")]
    UnterminatedString,
}

#[derive(Debug)]
pub struct ParseError(pub Token, pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.type_ {
            TokenType::Eof => write!(f, "[line {}] Error at end: {}", self.0.line, self.1),
            _ => write!(
                f,
                "[line {}] Error at {}: {}",
                self.0.line, self.0.lexeme, self.1
            ),
        }
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug)]
pub struct ResolveError;

pub type ResolveResult<T> = Result<T, ResolveError>;
