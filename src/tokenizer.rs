use crate::{errors::TokenizerError, tokens::*, types::Value};

pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: u32,
    had_eof: bool,
}

impl Scanner {
    pub fn new(source: String) -> Self {
        Self {
            source,
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
            _ => return None,
        })
    }

    fn from_type(&self, type_: TokenType) -> Token {
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

    fn get_token(&mut self) -> Result<Token, TokenizerError> {
        use TokenType::*;

        self.start = self.current;
        if self.is_at_end() {
            self.had_eof = true;
            return Ok(self.from_type(Eof));
        }

        let c = self.advance();
        match c {
            '(' => Ok(self.from_type(LeftParen)),
            ')' => Ok(self.from_type(RightParen)),
            '{' => Ok(self.from_type(LeftBrace)),
            '}' => Ok(self.from_type(RightBrace)),
            ',' => Ok(self.from_type(Comma)),
            '.' => Ok(self.from_type(Dot)),
            '-' => Ok(self.from_type(Minus)),
            '+' => Ok(self.from_type(Plus)),
            ';' => Ok(self.from_type(Semicolon)),
            '*' => Ok(self.from_type(Star)),
            '!' => Ok({
                let type_ = if self.match_('=') { BangEqual } else { Bang };
                self.from_type(type_)
            }),
            '=' => Ok({
                let type_ = if self.match_('=') { EqualEqual } else { Equal };
                self.from_type(type_)
            }),
            '>' => Ok({
                let type_ = if self.match_('=') {
                    GreaterEqual
                } else {
                    Greater
                };
                self.from_type(type_)
            }),
            '<' => Ok({
                let type_ = if self.match_('=') { LessEqual } else { Less };
                self.from_type(type_)
            }),
            '/' => {
                if self.match_('/') {
                    // We are reading a comment, skip to end of line
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                    Ok(self.from_type(Comment))
                } else {
                    Ok(self.from_type(Slash))
                }
            }
            ' ' | '\r' | '\t' => Ok(self.from_type(Whitespace)),
            '\n' => {
                self.line += 1;
                Ok(self.from_type(Whitespace))
            }
            '"' => {
                let string = self.string().ok_or(TokenizerError::UnterminatedString)?;
                Ok(self.new_token(String, Some(Value::String(string))))
            }
            c if c.is_ascii_digit() => {
                let number = self.number();
                Ok(self.new_token(Number, Some(Value::Number(number))))
            }
            c if c.is_ascii_alphabetic() => {
                while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
                    self.advance();
                }
                let keyword = self
                    .get_keyword(&self.source[self.start..self.current])
                    .unwrap_or(Identifier);
                Ok(self.from_type(keyword))
            }
            c => Err(TokenizerError::UnexpectedChar(c)),
        }
    }
}

impl Iterator for Scanner {
    type Item = Result<Token, TokenizerError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.had_eof {
            return None;
        }
        Some(self.get_token())
    }
}
