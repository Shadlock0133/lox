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
    #[error("None error")]
    NoneError,
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
    if parser.expression().is_some() {
        let line = parser.last_line;
        chunk.write(Opcode::RETURN, line);
        Ok(chunk)
    } else {
        let e = match parser.errors.len() {
            0 => Error::NoneError,
            1 => parser.errors.remove(0),
            _ => Error::MultipleErrors(parser.errors),
        };
        Err(e)
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

type ParseFn<'s, 'c> = fn(&mut Parser<'s, 'c>) -> Option<()>;

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
        TT::LeftParen    => Rule::new(Some(P::grouping), None, Zero),
        TT::RightParen   => Rule::new(None, None, Zero),
        TT::LeftBrace    => Rule::new(None, None, Zero),
        TT::RightBrace   => Rule::new(None, None, Zero),
        TT::Comma        => Rule::new(None, None, Zero),
        TT::Dot          => Rule::new(None, None, Zero),
        TT::Minus        => Rule::new(Some(P::unary), Some(P::binary), Term),
        TT::Plus         => Rule::new(None, Some(P::binary), Term),
        TT::Semicolon    => Rule::new(None, None, Zero),
        TT::Slash        => Rule::new(None, Some(P::binary), Factor),
        TT::Star         => Rule::new(None, Some(P::binary), Factor),
        TT::Bang         => Rule::new(None, None, Zero),
        TT::BangEqual    => Rule::new(None, None, Zero),
        TT::Equal        => Rule::new(None, None, Zero),
        TT::EqualEqual   => Rule::new(None, None, Zero),
        TT::Greater      => Rule::new(None, None, Zero),
        TT::GreaterEqual => Rule::new(None, None, Zero),
        TT::Less         => Rule::new(None, None, Zero),
        TT::LessEqual    => Rule::new(None, None, Zero),
        TT::Identifier   => Rule::new(None, None, Zero),
        TT::String       => Rule::new(None, None, Zero),
        TT::Number       => Rule::new(Some(P::number), None, Zero),
        TT::And          => Rule::new(None, None, Zero),
        TT::Class        => Rule::new(None, None, Zero),
        TT::Else         => Rule::new(None, None, Zero),
        TT::False        => Rule::new(None, None, Zero),
        TT::For          => Rule::new(None, None, Zero),
        TT::Fun          => Rule::new(None, None, Zero),
        TT::If           => Rule::new(None, None, Zero),
        TT::Nil          => Rule::new(None, None, Zero),
        TT::Or           => Rule::new(None, None, Zero),
        TT::Print        => Rule::new(None, None, Zero),
        TT::Return       => Rule::new(None, None, Zero),
        TT::Super        => Rule::new(None, None, Zero),
        TT::This         => Rule::new(None, None, Zero),
        TT::True         => Rule::new(None, None, Zero),
        TT::Var          => Rule::new(None, None, Zero),
        TT::While        => Rule::new(None, None, Zero),
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

    fn number(&mut self) -> Option<()> {
        let token = self.consume(TokenType::Number, "Expect a number.")?;
        let line = token.line;
        let value = token
            .lexeme
            .parse::<f64>()
            .map_err(|e| {
                let error = TokenError(token.to_owned(), e.to_string());
                self.error(error.into())
            })
            .ok()?;
        self.chunk.write_constant(Value(value), line);
        Some(())
    }

    fn grouping(&mut self) -> Option<()> {
        self.consume(TokenType::LeftParen, "Expect '(' before grouping.")?;
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after expression.")?;
        Some(())
    }

    fn unary(&mut self) -> Option<()> {
        let token = self.advance()?;
        self.parse_precedence(Precedence::Call)?;
        match token.type_ {
            TokenType::Minus => self.chunk.write(Opcode::NEGATE, token.line),
            _ => return None,
        }
        Some(())
    }

    fn binary(&mut self) -> Option<()> {
        let op = self.advance()?;
        let rule = get_rule(op.type_);
        self.parse_precedence(rule.precedence.left_assoc())?;
        match op.type_ {
            TokenType::Plus => self.chunk.write(Opcode::ADD, op.line),
            TokenType::Minus => self.chunk.write(Opcode::SUBSTRACT, op.line),
            TokenType::Star => self.chunk.write(Opcode::MULTIPLY, op.line),
            TokenType::Slash => self.chunk.write(Opcode::DIVIDE, op.line),
            _ => return None,
        }
        Some(())
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Option<()> {
        let token = self.peek()?;
        match get_rule(token.type_).prefix {
            Some(f) => f(self)?,
            None => {
                let token = token.clone().to_owned();
                self.errors.push(
                    TokenError(token, "Expect expression".to_string()).into(),
                );
                return None;
            }
        }

        loop {
            let token = match self.peek() {
                Some(t) => t,
                None => break
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

        Some(())
    }

    fn expression(&mut self) -> Option<()> {
        self.parse_precedence(Precedence::Assignment)
    }
}
