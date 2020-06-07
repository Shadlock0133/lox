use crate::{tokens::*, Reporter};
use std::{cell::RefCell, fmt, rc::Rc};

pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: u32,
    reporter: Rc<RefCell<Reporter>>,
    had_eof: bool,
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

impl Scanner {
    pub fn new(source: String, reporter: Rc<RefCell<Reporter>>) -> Self {
        Self {
            source,
            reporter,
            start: 0,
            current: 0,
            line: 1,
            had_eof: false,
        }
    }

    fn advance(&mut self) -> char {
        let char = self
            .source
            .get(self.current..)
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
        self.source
            .get(self.current..)
            .and_then(|x| x.chars().next())
            .unwrap_or('\0')
    }

    fn peek_next(&self) -> char {
        self.source
            .get(self.current..)
            .and_then(|x| x.chars().nth(1))
            .unwrap_or('\0')
    }

    // TODO: Add quote escaping for fun and profit
    fn string(&mut self) -> Option<String> {
        loop {
        // while self.peek() != '"' && !self.is_at_end() {
            if self.peek() != '\\' && self.peek_next() == '"' {
                self.advance();
                break;
            }
            if self.is_at_end() {
                break;
            }
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return None;
        }

        self.advance();
        Some(self.source[(self.start + 1)..(self.current - 1)].to_owned())
    }

    fn number(&mut self) -> f64 {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        self.source[self.start..self.current].parse().unwrap()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn get_keyword(&self, lexeme: &str) -> Option<TokenType> {
        use TokenType::*;

        Some(match lexeme {
            "and" => And,
            "break" => Break,
            "class" => Class,
            "else" => Else,
            "false" => False,
            "for" => For,
            "fun" => Fun,
            "if" => If,
            "nil" => Nil,
            "or" => Or,
            "print" => Print,
            "return" => Return,
            "super" => Super,
            "this" => This,
            "true" => True,
            "var" => Var,
            "while" => While,
            _ => None?,
        })
    }

    fn new_token_from_type(&self, type_: TokenType) -> Token {
        self.new_token(type_, None)
    }

    fn new_token(&self, type_: TokenType, literal: Option<Value>) -> Token {
        let lexeme = self.source[self.start..self.current].to_owned();
        Token {
            type_,
            literal,
            lexeme,
            line: self.line,
        }
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
                let type_ = if self.match_('=') {
                    GreaterEqual
                } else {
                    Greater
                };
                self.new_token_from_type(type_)
            })),
            '<' => Ok(Ok({
                let type_ = if self.match_('=') { LessEqual } else { Less };
                self.new_token_from_type(type_)
            })),
            '/' => {
                if self.match_('/') {
                    // We are reading a comment, skip to end of line
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                    Ok(Err(SkipToken::Comment))
                } else {
                    Ok(Ok(self.new_token_from_type(Slash)))
                }
            }
            ' ' | '\r' | '\t' => Ok(Err(SkipToken::Whitespace)),
            '\n' => {
                self.line += 1;
                Ok(Err(SkipToken::Whitespace))
            }
            '"' => {
                let string = self.string().ok_or(TokenError::UnterminatedString)?;
                Ok(Ok(self.new_token(String, Some(Value::String(string)))))
            }
            c if c.is_ascii_digit() => {
                let number = self.number();
                Ok(Ok(self.new_token(Number, Some(Value::Number(number)))))
            }
            c if c.is_ascii_alphabetic() => {
                while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
                    self.advance();
                }
                let keyword = self
                    .get_keyword(&self.source[self.start..self.current])
                    .unwrap_or(Identifier);
                Ok(Ok(self.new_token_from_type(keyword)))
            }
            c => Err(TokenError::UnexpectedChar(c)),
        }
    }
}

impl Iterator for Scanner {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        if self.had_eof {
            return None;
        }
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
            Ok(Ok(token)) => Some(token),
            _ => unreachable!(),
        }
    }
}
