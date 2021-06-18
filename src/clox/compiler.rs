use core::fmt;
use std::iter::Peekable;

use super::{
    chunk::{Chunk, ConstantIndex, Opcode},
    scanner::{self, Scanner, Token, TokenType},
    value::Value,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    ScannerError(#[from] scanner::Error),
    #[error("{0}")]
    ParserError(#[from] TokenError),
    #[error("{0}")]
    MultipleErrors(#[from] MulipleErrors),
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

#[derive(Debug, thiserror::Error)]
pub struct MulipleErrors(Vec<Error>);

impl fmt::Display for MulipleErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Multiple errors:")?;
        for error in &self.0 {
            write!(f, "\n    {}", error)?;
        }
        Ok(())
    }
}

pub fn compile(source: &str) -> Result<Chunk, Error> {
    let mut chunk = Chunk::default();
    let mut parser = Parser::new(&source, &mut chunk);
    let mut line = 0;
    while let Some(t) = parser.peek() {
        line = t.line;
        if let Err(()) = parser.declaration() {
            break;
        }
    }
    let mut errors = parser.errors;
    chunk.write(Opcode::RETURN, line);
    match errors.len() {
        0 => Ok(chunk),
        1 => Err(errors.remove(0)),
        _ => Err(MulipleErrors(errors).into()),
    }
}

struct Parser<'s, 'c> {
    scanner: Peekable<Scanner<'s>>,
    chunk: &'c mut Chunk,
    errors: Vec<Error>,
    panic_mode: bool,
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

type ParseFn<'s, 'c> = fn(&mut Parser<'s, 'c>, bool) -> Result<(), ()>;

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

fn get_rule<'s, 'c>(type_: TokenType) -> Rule<'s, 'c> {
    macro_rules! pratt_rules {
        (match $type:expr;
        $( $pat:ident => ( $prefix:ident, $infix:ident, $prec:ident ) ,)* ) => {
            match $type {
                $(
                    TokenType::$pat => Rule::new(
                        pratt_rules!(@fix $prefix),
                        pratt_rules!(@fix $infix),
                        Precedence::$prec
                    )
                ),*
            }
        };
        (@fix None) => { None };
        (@fix $f:ident) => { Some(Parser::$f) };
    }

    #[rustfmt::skip]
    pratt_rules!{match type_;
        LeftParen    => (   grouping,       None,       Zero),
        RightParen   => (       None,       None,       Zero),
        LeftBrace    => (       None,       None,       Zero),
        RightBrace   => (       None,       None,       Zero),
        Comma        => (       None,       None,       Zero),
        Dot          => (       None,       None,       Zero),
        Minus        => (      unary,     binary,       Term),
        Plus         => (       None,     binary,       Term),
        Semicolon    => (       None,       None,       Zero),
        Slash        => (       None,     binary,     Factor),
        Star         => (       None,     binary,     Factor),
        Bang         => (      unary,       None,       Zero),
        BangEqual    => (       None,     binary,   Equality),
        Equal        => (       None,       None,       Zero),
        EqualEqual   => (       None,     binary,   Equality),
        Greater      => (       None,     binary, Comparison),
        GreaterEqual => (       None,     binary, Comparison),
        Less         => (       None,     binary, Comparison),
        LessEqual    => (       None,     binary, Comparison),
        Identifier   => (   variable,       None,       Zero),
        String       => (     string,       None,       Zero),
        Number       => (     number,       None,       Zero),
        And          => (       None,       None,       Zero),
        Class        => (       None,       None,       Zero),
        Else         => (       None,       None,       Zero),
        False        => (    literal,       None,       Zero),
        For          => (       None,       None,       Zero),
        Fun          => (       None,       None,       Zero),
        If           => (       None,       None,       Zero),
        Nil          => (    literal,       None,       Zero),
        Or           => (       None,       None,       Zero),
        Print        => (       None,       None,       Zero),
        Return       => (       None,       None,       Zero),
        Super        => (       None,       None,       Zero),
        This         => (       None,       None,       Zero),
        True         => (    literal,       None,       Zero),
        Var          => (       None,       None,       Zero),
        While        => (       None,       None,       Zero),
    }
}

impl<'s, 'c> Parser<'s, 'c> {
    fn new(source: &'s str, chunk: &'c mut Chunk) -> Self {
        let scanner = Scanner::new(source).peekable();
        Self {
            scanner,
            chunk,
            errors: vec![],
            panic_mode: false,
            last_line: 0,
        }
    }

    fn emit(&mut self, bytes: &[u8], token: &Token) {
        for &byte in bytes {
            self.chunk.write(byte, token.line);
        }
    }

    fn error(&mut self, error: Error) -> Option<()> {
        if !self.panic_mode {
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

    // TODO: Probably want to correctly detect Eof
    fn match_(&mut self, type_: TokenType) -> Option<Token<'s>> {
        if self.peek()?.type_ == type_ {
            self.advance()
        } else {
            None
        }
    }

    fn consume(
        &mut self,
        type_: TokenType,
        message: &str,
    ) -> Option<Token<'s>> {
        let token = self.advance();
        match token {
            Some(token) if token.type_ == type_ => Some(token),
            Some(token) => {
                self.error(Error::ParserError(TokenError(
                    token.into_owned(),
                    message.to_string(),
                )));
                None
            }
            None => {
                self.error(Error::UnexpectedEof);
                None
            }
        }
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;
        while self.peek().is_some() {
            match self.peek().unwrap().type_ {
                TokenType::Semicolon => {
                    self.advance();
                    return;
                }
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,
                _ => (),
            }
            self.advance();
        }
    }

    fn number(&mut self, _can_assign: bool) -> Result<(), ()> {
        let token = self.advance().unwrap();
        let line = token.line;
        let value = token.lexeme.parse::<f64>().map_err(|e| {
            let error = TokenError(token.into_owned(), e.to_string());
            self.error(error.into());
        })?;
        self.chunk.write_constant(Value::number(value), line);
        Ok(())
    }

    fn string(&mut self, _can_assign: bool) -> Result<(), ()> {
        let token = self.advance().unwrap();
        let string = token.lexeme[1..token.lexeme.len() - 1].to_string();
        self.chunk.write_constant(Value::string(string), token.line);
        Ok(())
    }

    fn named_variable(
        &mut self,
        name: Token,
        can_assign: bool,
    ) -> Result<(), ()> {
        let line = name.line;
        let index = self.identifier_constant(name);

        if can_assign && self.match_(TokenType::Equal).is_some() {
            self.expression()?;
            self.chunk.set_global(index, line);
        } else {
            self.chunk.get_global(index, line);
        }

        Ok(())
    }

    fn variable(&mut self, can_assign: bool) -> Result<(), ()> {
        let name = self.advance().unwrap();
        self.named_variable(name, can_assign)
    }

    fn literal(&mut self, _can_assign: bool) -> Result<(), ()> {
        let token = self.advance().unwrap();
        match token.type_ {
            TokenType::Nil => self.chunk.write(Opcode::NIL, token.line),
            TokenType::True => self.chunk.write(Opcode::TRUE, token.line),
            TokenType::False => self.chunk.write(Opcode::FALSE, token.line),
            _ => return Err(()),
        }
        Ok(())
    }

    fn grouping(&mut self, _can_assign: bool) -> Result<(), ()> {
        self.advance().unwrap();
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after expression.")
            .ok_or(())?;
        Ok(())
    }

    fn unary(&mut self, _can_assign: bool) -> Result<(), ()> {
        let token = self.advance().unwrap();
        self.parse_precedence(Precedence::Call)?;
        match token.type_ {
            TokenType::Bang => self.chunk.write(Opcode::NOT, token.line),
            TokenType::Minus => self.chunk.write(Opcode::NEGATE, token.line),
            _ => return Err(()),
        }
        Ok(())
    }

    fn binary(&mut self, _can_assign: bool) -> Result<(), ()> {
        let op = self.advance().unwrap();
        let rule = get_rule(op.type_);
        self.parse_precedence(rule.precedence.left_assoc())?;
        match op.type_ {
            TokenType::BangEqual => {
                self.emit(&[Opcode::EQUAL, Opcode::NOT], &op)
            }
            TokenType::EqualEqual => self.emit(&[Opcode::EQUAL], &op),
            TokenType::Greater => self.emit(&[Opcode::GREATER], &op),
            TokenType::GreaterEqual => {
                self.emit(&[Opcode::LESS, Opcode::NOT], &op)
            }
            TokenType::Less => self.emit(&[Opcode::LESS], &op),
            TokenType::LessEqual => {
                self.emit(&[Opcode::GREATER, Opcode::NOT], &op)
            }
            TokenType::Plus => self.chunk.write(Opcode::ADD, op.line),
            TokenType::Minus => self.chunk.write(Opcode::SUBTRACT, op.line),
            TokenType::Star => self.chunk.write(Opcode::MULTIPLY, op.line),
            TokenType::Slash => self.chunk.write(Opcode::DIVIDE, op.line),
            _ => return Err(()),
        }
        Ok(())
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), ()> {
        let token = self.peek().ok_or(())?;
        let prefix = match get_rule(token.type_).prefix {
            Some(f) => f,
            None => {
                let token = token.clone().into_owned();
                self.error(
                    TokenError(token, "Expect expression".to_string()).into(),
                );
                return Err(());
            }
        };

        let can_assign = precedence <= Precedence::Assignment;
        prefix(self, can_assign)?;

        while let Some(token) = self.peek() {
            let rule = get_rule(token.type_);
            if precedence > rule.precedence {
                break;
            }
            if let Some(f) = rule.infix {
                f(self, can_assign)?
            }
        }

        if can_assign {
            if let Some(token) = self.match_(TokenType::Equal) {
                let token = token.clone().into_owned();
                self.error(
                    TokenError(token, "Invalid assignment target.".into())
                        .into(),
                );
            }
        }

        Ok(())
    }

    fn identifier_constant(&mut self, name: Token) -> ConstantIndex {
        self.chunk
            .add_constant(Value::string(name.lexeme.into_owned()))
    }

    fn parse_variable(&mut self, error_msg: &str) -> Result<ConstantIndex, ()> {
        let token = self.consume(TokenType::Identifier, error_msg).ok_or(())?;
        Ok(self.identifier_constant(token))
    }

    fn define_variable(&mut self, global: ConstantIndex, token: &Token) {
        self.chunk.define_global(global, token.line)
    }

    fn expression(&mut self) -> Result<(), ()> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn declaration(&mut self) -> Result<(), ()> {
        let ret = if let Some(var) = self.match_(TokenType::Var) {
            self.var_declaration(var)
        } else {
            self.statement()
        };
        if ret.is_err() {
            self.synchronize();
        }
        ret
    }

    fn var_declaration(&mut self, var: Token) -> Result<(), ()> {
        let global = self.parse_variable("Expect variable name.")?;
        if self.match_(TokenType::Equal).is_some() {
            self.expression()?;
        } else {
            self.chunk.write(Opcode::NIL, var.line);
        }
        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )
        .ok_or(())?;
        self.define_variable(global, &var);
        Ok(())
    }

    fn statement(&mut self) -> Result<(), ()> {
        if self.match_(TokenType::Print).is_some() {
            self.print_statement()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> Result<(), ()> {
        self.expression()?;
        let token = self
            .consume(TokenType::Semicolon, "Expect ';' after value.")
            .ok_or(())?;
        self.chunk.write(Opcode::PRINT, token.line);
        Ok(())
    }

    fn expression_statement(&mut self) -> Result<(), ()> {
        self.expression()?;
        let token = self
            .consume(TokenType::Semicolon, "Expect ';' after value.")
            .ok_or(())?;
        self.chunk.write(Opcode::POP, token.line);
        Ok(())
    }
}
