use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::Rc,
};
use crate::Reporter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TokenType {
    LeftParen, RightParen, LeftBrace, RightBrace,
    Comma, Dot, Minus, Plus, Semicolon, Slash, Star,

    Bang, BangEqual,
    Equal, EqualEqual,
    Greater, GreaterEqual,
    Less, LessEqual,

    Identifier, String, Number,

    And, Class, Else, False, Fun, For, If, Nil, Or,
    Print, Return, Super, This, True, Var, While,

    Eof,
}

#[derive(Debug)]
enum TokenError {
    UnexpectedChar(char),
    UnterminatedString,
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenError::UnterminatedString => write!(f, "Unterminated string"),
            TokenError::UnexpectedChar(ch) => write!(f, "Unexpected character: {}", ch),
        }
    }
}

#[derive(Debug)]
enum SkipToken {
    Comment,
    Whitespace,
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

impl Value {
    pub fn as_number(self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_string(self) -> Option<String> {
        match self {
            Value::String(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_bool(self) -> Option<bool> {
        match self {
            Value::Bool(n) => Some(n),
            _ => None,
        }
    }

    pub fn is_truthy(self) -> bool {
        match self {
            Value::Bool(b) => b,
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

impl Token {
    pub fn new(type_: TokenType, lexeme: String, literal: Option<Value>, line: u32) -> Self {
        Self { type_, lexeme, literal, line, }
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

pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: u32,
    reporter: Rc<RefCell<Reporter>>,
    keywords: HashMap<String, TokenType>,
    had_eof: bool,
}

impl Scanner {
    pub fn new(source: String, reporter: Rc<RefCell<Reporter>>) -> Self {
        let mut keywords = HashMap::new();
        keywords.insert("and".into(), TokenType::And);
        keywords.insert("class".into(), TokenType::Class);
        keywords.insert("else".into(), TokenType::Else);
        keywords.insert("false".into(), TokenType::False);
        keywords.insert("for".into(), TokenType::For);
        keywords.insert("fun".into(), TokenType::Fun);
        keywords.insert("if".into(), TokenType::If);
        keywords.insert("nil".into(), TokenType::Nil);
        keywords.insert("or".into(), TokenType::Or);
        keywords.insert("print".into(), TokenType::Print);
        keywords.insert("return".into(), TokenType::Return);
        keywords.insert("super".into(), TokenType::Super);
        keywords.insert("this".into(), TokenType::This);
        keywords.insert("true".into(), TokenType::True);
        keywords.insert("var".into(), TokenType::Var);
        keywords.insert("while".into(), TokenType::While);

        Self {
            source, reporter, keywords,
            start: 0,
            current: 0,
            line: 1,
            had_eof: false,
        }
    }

    fn advance(&mut self) -> char {
        let char = self.source.get(self.current..)
            .and_then(|x| x.chars().next())
            .unwrap_or('\0');
        self.current += char.len_utf8();
        char
    }

    fn match_(&mut self, expected: char) -> bool {
        let char = self.source.chars().nth(self.current).unwrap_or('\0');
        let is_match = !self.is_at_end() && char == expected;
        if is_match {
            self.current += char.len_utf8();
        }
        is_match
    }

    fn peek(&self) -> char {
        self.source.get(self.current..)
            .and_then(|x| x.chars().next())
            .unwrap_or('\0')
    }

    fn peek_next(&self) -> char {
        self.source.get(self.current..)
            .and_then(|x| x.chars().nth(1))
            .unwrap_or('\0')
    }

    // TODO: Add quote escaping for fun and profit
    fn string(&mut self) -> Option<String> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' { self.line += 1; }
            self.advance();
        }

        if self.is_at_end() {
            return None;
        }

        self.advance();
        Some(self.source[(self.start + 1)..(self.current - 1)].to_owned())
    }

    fn number(&mut self) -> f64 {
        while self.peek().is_ascii_digit() { self.advance(); }

        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();
            while self.peek().is_ascii_digit() { self.advance(); }
        }

        self.source[self.start..self.current].parse().unwrap()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn new_token_from_type(&self, type_: TokenType) -> Token {
        self.new_token(type_, None)
    }

    fn new_token(&self, type_: TokenType, literal: Option<Value>) -> Token {
        let lexeme = self.source[self.start..self.current].to_owned();
        Token { type_, literal, lexeme, line: self.line, }
    }

    fn get_token(&mut self) -> Result<Result<Token, SkipToken>, TokenError> {
        use TokenType::*;

        self.start = self.current;
        if self.is_at_end() {
            self.had_eof = true;
            return Ok(Ok(self.new_token_from_type(Eof)));
        }

        let c = self.advance();
        match c {
            '(' => Ok(Ok(self.new_token_from_type(LeftParen))),
            ')' => Ok(Ok(self.new_token_from_type(RightParen))),
            '{' => Ok(Ok(self.new_token_from_type(LeftBrace))),
            '}' => Ok(Ok(self.new_token_from_type(RightBrace))),
            ',' => Ok(Ok(self.new_token_from_type(Comma))),
            '.' => Ok(Ok(self.new_token_from_type(Dot))),
            '-' => Ok(Ok(self.new_token_from_type(Minus))),
            '+' => Ok(Ok(self.new_token_from_type(Plus))),
            ';' => Ok(Ok(self.new_token_from_type(Semicolon))),
            '*' => Ok(Ok(self.new_token_from_type(Star))),
            '!' => Ok(Ok({
                let type_ = if self.match_('=') { BangEqual } else { Bang };
                self.new_token_from_type(type_)
            })),
            '=' => Ok(Ok({
                let type_ = if self.match_('=') { EqualEqual } else { Equal };
                self.new_token_from_type(type_)
            })),
            '>' => Ok(Ok({
                let type_ = if self.match_('=') { GreaterEqual } else { Greater };
                self.new_token_from_type(type_)
            })),
            '<' => Ok(Ok({
                let type_ = if self.match_('=') { LessEqual } else { Less };
                self.new_token_from_type(type_)
            })),
            '/' => {
                if self.match_('/') {
                    // We are reading a comment, skip to end of line
                    while self.peek() != '\n' && !self.is_at_end() { self.advance(); }
                    Ok(Err(SkipToken::Comment))
                } else {
                    Ok(Ok(self.new_token_from_type(Slash)))
                }
            }
            ' ' | '\r' | '\t' => Ok(Err(SkipToken::Whitespace)),
            '\n' => { self.line += 1; Ok(Err(SkipToken::Whitespace)) },
            '"' => {
                let string = self.string().ok_or(TokenError::UnterminatedString)?;
                Ok(Ok(self.new_token(String, Some(Value::String(string)))))
            },
            c if c.is_ascii_digit() => {
                let number = self.number();
                Ok(Ok(self.new_token(Number, Some(Value::Number(number)))))
            }
            c if c.is_ascii_alphabetic() => {
                while self.peek().is_ascii_alphanumeric() || self.peek() == '_' { self.advance(); }
                let keyword = self.keywords.get(&self.source[self.start..self.current]).unwrap_or(&Identifier);
                Ok(Ok(self.new_token_from_type(*keyword)))
            }
            c => Err(TokenError::UnexpectedChar(c)),
        }
    }
}

impl Iterator for Scanner {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        if self.had_eof { return None; }
        let mut token = self.get_token();
        loop {
            match token {
                Err(err) => {
                    self.reporter
                        .borrow_mut()
                        .error(self.line, format!("{}", err));
                    token = self.get_token();
                    continue;
                }
                Ok(Err(_)) => {
                    token = self.get_token();
                    continue;
                }
                _ => break,
            }
        }
        match token {
            Ok(Ok(token)) => {
                // eprintln!("> {}", token);
                Some(token)
            },
            _ => unreachable!(),
        }
    }
}