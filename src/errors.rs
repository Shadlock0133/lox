use crate::{
    tokens::{Token, TokenType},
    types::Value,
};

#[derive(Debug)]
pub struct GenericError(pub Option<Token>, pub String);

impl GenericError {
    fn to_string(&self, kind: &'static str) -> String {
        match &self.0 {
            Some(token) => {
                let lexeme = match token.type_ {
                    TokenType::Eof => "end",
                    _ => &token.lexeme,
                };
                format!(
                    "[line {}:{}] {}Error at '{}': {}",
                    token.pos.0, token.pos.1, kind, lexeme, self.1
                )
            }
            None => format!("{}Error: {}", kind, self.1),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Unexpected return")]
    Return(Value),
    #[error("Unexpected break")]
    Break,
    #[error("{}", _0.to_string("Runtime "))]
    Error(GenericError),
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

impl RuntimeError {
    pub fn new<S: Into<String>>(token: Option<&Token>, message: S) -> Self {
        Self::Error(GenericError(token.cloned(), message.into()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenizerError {
    #[error("Unexpected character: {0}")]
    UnexpectedChar(char),
    #[error("Unterminated string")]
    UnterminatedString,
}

#[derive(Debug, thiserror::Error)]
#[error("{}", self.0.to_string("Parse "))]
pub struct ParseError(pub GenericError);

impl ParseError {
    pub fn new(token: Option<Token>, msg: String) -> Self {
        Self(GenericError(token, msg))
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, thiserror::Error)]
#[error("{}", self.0.to_string("Resolve "))]
pub struct ResolveError(pub GenericError);

impl ResolveError {
    pub fn new(token: Option<Token>, msg: impl Into<String>) -> Self {
        Self(GenericError(token, msg.into()))
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;
