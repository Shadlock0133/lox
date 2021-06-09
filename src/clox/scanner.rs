use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    // Single-character tokens.
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
    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals.
    Identifier,
    String,
    Number,
    // Keywords.
    And,
    Class,
    Else,
    False,
    For,
    Fun,
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
}

#[derive(Debug, Clone)]
pub struct Token<'s> {
    pub type_: TokenType,
    pub lexeme: Cow<'s, str>,
    pub line: usize,
}

impl Token<'_> {
    pub fn into_owned(self) -> Token<'static> {
        let Token {
            type_,
            lexeme,
            line,
        } = self;
        Token {
            type_,
            lexeme: lexeme.into_owned().into(),
            line,
        }
    }
}

pub struct Scanner<'s> {
    source: &'s str,
    start: usize,
    current: usize,
    line: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("Unexpected character")]
    UnexpectedCharacter,
    #[error("Unterminated string")]
    UnterminatedString,
}

#[derive(Debug, thiserror::Error)]
#[error("Scanner error: {kind} at line {line}")]
pub struct Error {
    kind: ErrorKind,
    line: usize,
}

impl Error {
    fn new(kind: ErrorKind, line: usize) -> Self {
        Self { kind, line }
    }
}

impl<'s> Iterator for Scanner<'s> {
    type Item = Result<Token<'s>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next().transpose()
    }
}

impl<'s> Scanner<'s> {
    pub fn new(source: &'s str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    fn next(&mut self) -> Result<Option<Token<'s>>, Error> {
        self.skip_whitespace();
        self.start = self.current;
        let c = match self.advance() {
            Some(c) => c,
            None => return Ok(None),
        };
        let token = match c {
            '(' => self.token(TokenType::LeftParen),
            ')' => self.token(TokenType::RightParen),
            '{' => self.token(TokenType::LeftBrace),
            '}' => self.token(TokenType::RightBrace),
            ';' => self.token(TokenType::Semicolon),
            ',' => self.token(TokenType::Comma),
            '.' => self.token(TokenType::Dot),
            '-' => self.token(TokenType::Minus),
            '+' => self.token(TokenType::Plus),
            '/' => self.token(TokenType::Slash),
            '*' => self.token(TokenType::Star),
            '!' => {
                if self.match_('=') {
                    self.token(TokenType::BangEqual)
                } else {
                    self.token(TokenType::Bang)
                }
            }
            '=' => {
                if self.match_('=') {
                    self.token(TokenType::EqualEqual)
                } else {
                    self.token(TokenType::Equal)
                }
            }
            '<' => {
                if self.match_('=') {
                    self.token(TokenType::LessEqual)
                } else {
                    self.token(TokenType::Less)
                }
            }
            '>' => {
                if self.match_('=') {
                    self.token(TokenType::GreaterEqual)
                } else {
                    self.token(TokenType::Greater)
                }
            }
            '"' => self.string()?,
            '0'..='9' => self.number(),
            'a'..='z' | 'A'..='Z' | '_' => self.identifier(),
            _ => {
                return Err(Error::new(
                    ErrorKind::UnexpectedCharacter,
                    self.line,
                ))
            }
        };
        Ok(Some(token))
    }

    fn lexeme(&self) -> &'s str {
        &self.source[self.start..self.current]
    }

    fn token(&self, type_: TokenType) -> Token<'s> {
        Token {
            type_,
            lexeme: self.lexeme().into(),
            line: self.line,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.current..)?.chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        self.source.get(self.current..)?.chars().nth(1)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.current += c.len_utf8();
        Some(c)
    }

    fn match_(&mut self, expected: char) -> bool {
        match self.peek() {
            Some(c) if c == expected => {
                self.current += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\r') | Some('\t') => {
                    self.advance();
                }
                Some('\n') => {
                    self.advance();
                    self.line += 1;
                }
                Some('/') => {
                    if self.peek_next() == Some('/') {
                        while self.peek().filter(|x| *x != '\n').is_some() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    fn string(&mut self) -> Result<Token<'s>, Error> {
        while self.peek().filter(|x| *x != '"').is_some() {
            if self.peek() == Some('\n') {
                self.line += 1;
            }
            self.advance();
        }
        if self.peek().is_none() {
            Err(Error::new(ErrorKind::UnterminatedString, self.line))
        } else {
            self.advance();
            Ok(self.token(TokenType::String))
        }
    }

    fn number(&mut self) -> Token<'s> {
        while matches!(self.peek(), Some('0'..='9')) {
            self.advance();
        }
        if self.peek() == Some('.')
            && matches!(self.peek_next(), Some('0'..='9'))
        {
            self.advance();
            while matches!(self.peek(), Some('0'..='9')) {
                self.advance();
            }
        }
        self.token(TokenType::Number)
    }

    fn identifier(&mut self) -> Token<'s> {
        while self
            .peek()
            .filter(|x| x.is_ascii_alphanumeric() || *x == '_')
            .is_some()
        {
            self.advance();
        }
        let type_ = match self.lexeme() {
            "and" => TokenType::And,
            "class" => TokenType::Class,
            "else" => TokenType::Else,
            "false" => TokenType::False,
            "for" => TokenType::For,
            "fun" => TokenType::Fun,
            "if" => TokenType::If,
            "nil" => TokenType::Nil,
            "or" => TokenType::Or,
            "print" => TokenType::Print,
            "return" => TokenType::Return,
            "super" => TokenType::Super,
            "this" => TokenType::This,
            "true" => TokenType::True,
            "var" => TokenType::Var,
            "while" => TokenType::While,
            _ => TokenType::Identifier,
        };
        self.token(type_)
    }
}
