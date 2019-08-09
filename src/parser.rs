use crate::{
    syntax::*,
    tokens::{
        Token,
        TokenType::{self, *},
        Value,
    },
    Reporter,
};
use std::{cell::RefCell, rc::Rc};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    reporter: Rc<RefCell<Reporter>>,
}

#[derive(Debug)]
pub struct ParseError;

impl Parser {
    pub fn new(tokens: Vec<Token>, reporter: Rc<RefCell<Reporter>>) -> Self {
        Self {
            tokens,
            reporter,
            current: 0,
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek().type_ == Eof
    }

    fn peek(&self) -> Token {
        self.tokens[self.current].clone()
    }

    fn previous(&mut self) -> Token {
        self.tokens[self.current - 1].clone()
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&mut self, type_: TokenType) -> bool {
        if !self.is_at_end() {
            self.peek().type_ == type_
        } else {
            false
        }
    }

    // It must take all possible types because we often check
    // for match on multiple types;
    // if we compared one by one outside match_ we might end up
    // consumming multiple tokens instead of one
    // eg. equality on "foo bar != == baz" would consume both "!= =="
    fn match_(&mut self, types: &[TokenType]) -> bool {
        for type_ in types {
            if self.check(*type_) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn error<S: AsRef<str>>(&mut self, token: Token, message: S) -> ParseError {
        self.reporter.borrow_mut().with_token(token, message);
        ParseError
    }

    fn consume<S: AsRef<str>>(
        &mut self,
        type_: TokenType,
        message: S,
    ) -> Result<Token, ParseError> {
        if self.check(type_) {
            return Ok(self.advance());
        }
        Err(self.error(self.peek(), message))
    }

    fn synchronize(&mut self) {
        self.advance();
        while !self.is_at_end() {
            if self.previous().type_ == Semicolon {
                return;
            }
            match self.peek().type_ {
                Class | Fun | Var | For | If | While | Print | Return => return,
                _ => (),
            }
            self.advance();
        }
    }

    pub fn parse(&mut self) -> Result<Expr, ParseError> {
        self.expression()
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;

        while self.match_(&[BangEqual, EqualEqual]) {
            let token = self.previous();
            let right = self.comparison()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.addition()?;

        while self.match_(&[Greater, GreaterEqual, Less, LessEqual]) {
            let token = self.previous();
            let right = self.addition()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn addition(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.multiplication()?;

        while self.match_(&[Minus, Plus]) {
            let token = self.previous();
            let right = self.multiplication()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn multiplication(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;

        while self.match_(&[Slash, Star]) {
            let token = self.previous();
            let right = self.unary()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_(&[Bang, Minus]) {
            let token = self.previous();
            let right = self.unary()?;
            Ok(Expr::unary(token, right))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        if self.match_(&[False]) {
            return Ok(Expr::literal(Value::Bool(false)));
        }
        if self.match_(&[True]) {
            return Ok(Expr::literal(Value::Bool(true)));
        }
        if self.match_(&[Nil]) {
            return Ok(Expr::literal(Value::Nil));
        }
        if self.match_(&[Number, String]) {
            return Ok(Expr::literal(
                self.previous()
                    .literal
                    .ok_or_else(|| self.error(self.peek(), "Missing literal"))?,
            ));
        }
        if self.match_(&[LeftParen]) {
            let expr = self.expression()?;
            self.consume(RightParen, "Expect ')' after expression.")?;
            return Ok(Expr::grouping(expr));
        }
        Err(self.error(self.peek(), "Not a valid expression"))
    }
}
