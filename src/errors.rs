use crate::{
    tokens::{Token, TokenType},
    types::ValueRef,
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
pub enum ControlFlow {
    #[error("Unexpected return")]
    Return(ValueRef),
    #[error("Unexpected break")]
    Break,
    #[error("{0}")]
    Error(RuntimeError),
}

#[derive(Debug, thiserror::Error)]
#[error("{}", _0.to_string("Runtime "))]
pub struct RuntimeError(GenericError);

pub type RuntimeResult<T> = Result<T, ControlFlow>;

impl RuntimeError {
    pub fn new<S: Into<String>>(
        token: Option<&Token>,
        message: S,
    ) -> ControlFlow {
        ControlFlow::Error(Self(GenericError(token.cloned(), message.into())))
    }
}

impl ControlFlow {
    pub fn into_error(self) -> RuntimeError {
        match self {
            ControlFlow::Return(value) => RuntimeError(GenericError(
                None,
                format!("Unexpected return: {}", value.value()),
            )),
            ControlFlow::Break => {
                RuntimeError(GenericError(None, "Unexpected break".to_string()))
            }
            ControlFlow::Error(err) => err,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenizerError {
    #[error("Unexpected character.")]
    UnexpectedChar(char),
    #[error("Unterminated string.")]
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
    pub fn new(token: Option<&Token>, msg: impl Into<String>) -> Self {
        Self(GenericError(token.cloned(), msg.into()))
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;
