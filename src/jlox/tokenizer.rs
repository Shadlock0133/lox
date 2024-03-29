use super::{errors::TokenizerError, tokens::*, types::Value};

pub struct Tokenizer<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    line_pos: (u32, u32),
    had_eof: bool,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line_pos: (1, 0),
            had_eof: false,
        }
    }

    fn advance(&mut self) -> char {
        let char = self.peek();
        self.current += char.len_utf8();
        self.line_pos.1 += 1;
        char
    }

    fn match_(&mut self, expected: char) -> bool {
        let char = self.peek();
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
        let mut output = String::new();
        if self.peek() != '"' {
            loop {
                if self.peek() != '\r' {
                    output.push(self.peek());
                }
                if self.peek() != '\\' && self.peek_next() == '"' {
                    self.advance();
                    break;
                }
                if self.is_at_end() {
                    break;
                }
                if self.peek() == '\n' {
                    self.line_pos.0 += 1;
                    self.line_pos.1 = 0;
                }
                self.advance();
            }
        }

        if self.is_at_end() {
            return None;
        }

        self.advance();
        Some(output)
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
        let lexeme = self.source[self.start..self.current].into();
        Token {
            type_,
            literal,
            lexeme,
            pos: self.line_pos,
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
                self.line_pos.0 += 1;
                self.line_pos.1 = 0;
                Ok(self.from_type(Whitespace))
            }
            '"' => {
                let string =
                    self.string().ok_or(TokenizerError::UnterminatedString)?;
                Ok(self.new_token(String, Some(Value::String(string))))
            }
            c if c.is_ascii_digit() => {
                let number = self.number();
                Ok(self.new_token(Number, Some(Value::Number(number))))
            }
            c if c.is_ascii_alphabetic() => {
                while self.peek().is_ascii_alphanumeric() || self.peek() == '_'
                {
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

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token, TokenizerError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.had_eof {
            return None;
        }
        Some(self.get_token())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let run = |x: &str| {
            Tokenizer::new(x.into())
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        assert_eq!(run(r#""test""#)[0].type_, TokenType::String);
        assert_eq!(run(r#""test""#)[0].lexeme, "\"test\"");

        assert_eq!(run("123")[0].type_, TokenType::Number);
        assert_eq!(run("-123.2")[0].type_, TokenType::Minus);
        assert_eq!(run("-123.2")[1].type_, TokenType::Number);
        assert_eq!(run("-123.2")[1].lexeme, "123.2");
        assert_eq!(run("-123.2")[2].type_, TokenType::Eof);

        assert_eq!(run("true")[0].type_, TokenType::True);
        assert_eq!(run("false")[0].type_, TokenType::False);

        assert_eq!(run(" \r\t\n ")[0].type_, TokenType::Whitespace);
        assert_eq!(run(" \r\t\n ")[1].type_, TokenType::Whitespace);
        assert_eq!(run(" \r\t\n ")[2].type_, TokenType::Whitespace);
        assert_eq!(run(" \r\t\n ")[3].type_, TokenType::Whitespace);
        assert_eq!(run(" \r\t\n ")[4].type_, TokenType::Whitespace);
        assert_eq!(run(" \r\t\n ")[5].type_, TokenType::Eof);
    }
}
