use std::fmt;

use super::types::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    Comment,
    Whitespace,

    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub type_: TokenType,
    pub lexeme: String,
    pub literal: Option<Value>,
    pub pos: (u32, u32),
}

impl Token {
    pub fn can_skip(&self) -> bool {
        matches!(self.type_, TokenType::Comment | TokenType::Whitespace)
    }
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
