use core::fmt;
use std::iter::Peekable;

use super::{
    chunk::{Chunk, Opcode},
    scanner::{self, Scanner, Token, TokenType},
    value::Value,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    ScannerError(#[from] scanner::Error),
    #[error("{0}")]
    ParserError(#[from] TokenError),
    #[error("Multiple errors: {0:?}")]
    MultipleErrors(Vec<Error>),
    #[error("Unexpected EOF")]
    UnexpectedEof,
}

#[derive(Debug, thiserror::Error)]
pub struct TokenError(Token<'static>, String);

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Line {}] Parser error at '{}': {}",
            self.0.line, self.0.lexeme, self.1
        )
    }
}

pub fn compile(source: &str) -> Result<Chunk, Error> {
    let mut chunk = Chunk::default();
    let mut parser = Parser::new(&source, &mut chunk);
    match parser.expression() {
        Ok(()) => {
            let line = parser.last_line;
            chunk.write(Opcode::RETURN, line);
            Ok(chunk)
        }
        Err(()) => {
            let e = match parser.errors.len() {
                0 => Error::UnexpectedEof,
                1 => parser.errors.remove(0),
                _ => Error::MultipleErrors(parser.errors),
            };
            Err(e)
        }
    }
}

struct Parser<'s, 'c> {
    scanner: Peekable<Scanner<'s>>,
    chunk: &'c mut Chunk,
    errors: Vec<Error>,
    synchronizing: bool,
    last_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum Precedence {
    Zero,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    fn left_assoc(self) -> Self {
        match self {
            Precedence::Zero => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => unreachable!(),
        }
    }
}

type ParseFn<'s, 'c> = fn(&mut Parser<'s, 'c>) -> Result<(), ()>;

struct Rule<'s, 'c> {
    prefix: Option<ParseFn<'s, 'c>>,
    infix: Option<ParseFn<'s, 'c>>,
    precedence: Precedence,
}

impl<'s, 'c> Rule<'s, 'c> {
    fn new(
        prefix: Option<ParseFn<'s, 'c>>,
        infix: Option<ParseFn<'s, 'c>>,
        precedence: Precedence,
    ) -> Self {
        Self {
            prefix,
            infix,
            precedence,
        }
    }
}

#[rustfmt::skip]
fn get_rule<'s, 'c>(type_: TokenType) -> Rule<'s, 'c> {
    use {Parser as P, Precedence::*, TokenType as TT};
    match type_ {
        TT::LeftParen    => Rule::new(Some(P::grouping),            None, Zero),
        TT::RightParen   => Rule::new(             None,            None, Zero),
        TT::LeftBrace    => Rule::new(             None,            None, Zero),
        TT::RightBrace   => Rule::new(             None,            None, Zero),
        TT::Comma        => Rule::new(             None,            None, Zero),
        TT::Dot          => Rule::new(             None,            None, Zero),
        TT::Minus        => Rule::new(   Some(P::unary), Some(P::binary), Term),
        TT::Plus         => Rule::new(             None, Some(P::binary), Term),
        TT::Semicolon    => Rule::new(             None,            None, Zero),
        TT::Slash        => Rule::new(             None, Some(P::binary), Factor),
        TT::Star         => Rule::new(             None, Some(P::binary), Factor),
        TT::Bang         => Rule::new(   Some(P::unary),            None, Zero),
        TT::BangEqual    => Rule::new(             None, Some(P::binary), Equality),
        TT::Equal        => Rule::new(             None,            None, Zero),
        TT::EqualEqual   => Rule::new(             None, Some(P::binary), Equality),
        TT::Greater      => Rule::new(             None, Some(P::binary), Comparison),
        TT::GreaterEqual => Rule::new(             None, Some(P::binary), Comparison),
        TT::Less         => Rule::new(             None, Some(P::binary), Comparison),
        TT::LessEqual    => Rule::new(             None, Some(P::binary), Comparison),
        TT::Identifier   => Rule::new(             None,            None, Zero),
        TT::String       => Rule::new(             None,            None, Zero),
        TT::Number       => Rule::new(  Some(P::number),            None, Zero),
        TT::And          => Rule::new(             None,            None, Zero),
        TT::Class        => Rule::new(             None,            None, Zero),
        TT::Else         => Rule::new(             None,            None, Zero),
        TT::False        => Rule::new( Some(P::literal),            None, Zero),
        TT::For          => Rule::new(             None,            None, Zero),
        TT::Fun          => Rule::new(             None,            None, Zero),
        TT::If           => Rule::new(             None,            None, Zero),
        TT::Nil          => Rule::new( Some(P::literal),            None, Zero),
        TT::Or           => Rule::new(             None,            None, Zero),
        TT::Print        => Rule::new(             None,            None, Zero),
        TT::Return       => Rule::new(             None,            None, Zero),
        TT::Super        => Rule::new(             None,            None, Zero),
        TT::This         => Rule::new(             None,            None, Zero),
        TT::True         => Rule::new( Some(P::literal),            None, Zero),
        TT::Var          => Rule::new(             None,            None, Zero),
        TT::While        => Rule::new(             None,            None, Zero),
    }
}

impl<'s, 'c> Parser<'s, 'c> {
    fn new(source: &'s str, chunk: &'c mut Chunk) -> Self {
        let scanner = Scanner::new(source).peekable();
        Self {
            scanner,
            chunk,
            errors: vec![],
            synchronizing: false,
            last_line: 0,
        }
    }

    fn emit(&mut self, bytes: &[u8], line: usize) {
        for &byte in bytes {
            self.chunk.write(byte, line);
        }
    }

    fn error(&mut self, error: Error) -> Option<()> {
        if !self.synchronizing {
            self.errors.push(error);
        }
        None
    }

    // Consumes from scanner until it hits non-Err
    fn consume_errors(&mut self) {
        while matches!(self.scanner.peek(), Some(Err(_))) {
            let e = self.scanner.next().unwrap().unwrap_err();
            self.error(e.into());
        }
    }

    fn peek(&mut self) -> Option<&Token<'s>> {
        self.consume_errors();
        self.scanner.peek().map(Result::as_ref).transpose().unwrap()
    }

    fn advance(&mut self) -> Option<Token<'s>> {
        self.consume_errors();
        let token = self.scanner.next().transpose().unwrap()?;
        self.last_line = token.line;
        Some(token)
    }

    fn consume(
        &mut self,
        type_: TokenType,
        message: &str,
    ) -> Option<Token<'s>> {
        let token = self.advance()?;
        if token.type_ == type_ {
            Some(token)
        } else {
            self.error(Error::ParserError(TokenError(
                token.to_owned(),
                message.to_string(),
            )));
            None
        }
    }

    fn number(&mut self) -> Result<(), ()> {
        let token = self.advance().unwrap();
        let line = token.line;
        let value = token.lexeme.parse::<f64>().map_err(|e| {
            let error = TokenError(token.to_owned(), e.to_string());
            self.error(error.into());
        })?;
        self.chunk.write_constant(Value::number(value), line);
        Ok(())
    }

    fn literal(&mut self) -> Result<(), ()> {
        let token = self.advance().unwrap();
        match token.type_ {
            TokenType::Nil => self.chunk.write(Opcode::NIL, token.line),
            TokenType::True => self.chunk.write(Opcode::TRUE, token.line),
            TokenType::False => self.chunk.write(Opcode::FALSE, token.line),
            _ => return Err(()),
        }
        Ok(())
    }

    fn grouping(&mut self) -> Result<(), ()> {
        self.advance().unwrap();
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after expression.")
            .ok_or(())?;
        Ok(())
    }

    fn unary(&mut self) -> Result<(), ()> {
        let token = self.advance().unwrap();
        self.parse_precedence(Precedence::Call)?;
        match token.type_ {
            TokenType::Bang => self.chunk.write(Opcode::NOT, token.line),
            TokenType::Minus => self.chunk.write(Opcode::NEGATE, token.line),
            _ => return Err(()),
        }
        Ok(())
    }

    fn binary(&mut self) -> Result<(), ()> {
        let op = self.advance().unwrap();
        let rule = get_rule(op.type_);
        self.parse_precedence(rule.precedence.left_assoc())?;
        match op.type_ {
            TokenType::BangEqual => {
                self.emit(&[Opcode::EQUAL, Opcode::NOT], op.line)
            }
            TokenType::EqualEqual => self.emit(&[Opcode::EQUAL], op.line),
            TokenType::Greater => self.emit(&[Opcode::GREATER], op.line),
            TokenType::GreaterEqual => {
                self.emit(&[Opcode::LESS, Opcode::NOT], op.line)
            }
            TokenType::Less => self.emit(&[Opcode::LESS], op.line),
            TokenType::LessEqual => {
                self.emit(&[Opcode::GREATER, Opcode::NOT], op.line)
            }
            TokenType::Plus => self.chunk.write(Opcode::ADD, op.line),
            TokenType::Minus => self.chunk.write(Opcode::SUBSTRACT, op.line),
            TokenType::Star => self.chunk.write(Opcode::MULTIPLY, op.line),
            TokenType::Slash => self.chunk.write(Opcode::DIVIDE, op.line),
            _ => return Err(()),
        }
        Ok(())
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), ()> {
        let token = self.peek().ok_or(())?;
        match get_rule(token.type_).prefix {
            Some(f) => f(self)?,
            None => {
                let token = token.clone().to_owned();
                self.errors.push(
                    TokenError(token, "Expect expression".to_string()).into(),
                );
                return Err(());
            }
        }

        loop {
            let token = match self.peek() {
                Some(t) => t,
                None => break,
            };
            let rule = get_rule(token.type_);
            if precedence > rule.precedence {
                break;
            }
            match rule.infix {
                Some(f) => f(self)?,
                None => {}
            }
        }

        Ok(())
    }

    fn expression(&mut self) -> Result<(), ()> {
        self.parse_precedence(Precedence::Assignment)
    }
}
