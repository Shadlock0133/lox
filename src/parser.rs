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

    fn error<S: Into<std::string::String>>(&mut self, token: Token, message: S) -> ParseError {
        self.reporter.borrow_mut().with_token(token, message);
        ParseError
    }

    fn consume<S: Into<std::string::String>>(
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

    pub fn parse(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        Ok(statements)
    }

    fn declaration(&mut self) -> Result<Stmt, ParseError> {
        (|| {
            if self.match_(&[Var]) {
                return self.var_declaration();
            }
            self.statement()
        })()
        .map_err(|x| {
            self.synchronize();
            x
        })
    }

    fn var_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self.consume(Identifier, "Expect variable name")?;
        let init = if self.match_(&[Equal]) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(Semicolon, "Expect ';' after variable statement")?;
        Ok(Stmt::var(name, init))
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_(&[If]) {
            self.if_statement()
        } else if self.match_(&[Print]) {
            self.print_statement()
        } else if self.match_(&[LeftBrace]) {
            Ok(Stmt::block(self.block()?))
        }
        else {
            self.expression_statement()
        }
    }

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(LeftParen, "Expect '(' after if.")?;
        let condition = self.expression()?;
        self.consume(RightParen, "Expect ')' after if condition.")?;

        let then_branch = self.statement()?;
        let else_branch = if self.match_(&[Else]) {
            Some(self.statement()?)
        } else {
            None
        };

        Ok(Stmt::if_(condition, then_branch, else_branch))
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();

        while !self.check(RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        self.consume(RightBrace, "Expect '}' after block.")?;

        Ok(statements)
    }

    fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression()?;
        self.consume(Semicolon, "Expect ';' after value")?;
        Ok(Stmt::print(expr))
    }

    fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression()?;
        self.consume(Semicolon, "Expect ';' after value")?;
        Ok(Stmt::expression(expr))
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.or()?;

        if self.match_(&[Equal]) {
            let equals = self.previous();
            let value = self.assignment()?;

            if let Expr::Variable(variable) = expr {
                return Ok(Expr::assign(variable.name, value));
            }
            self.error(equals, "Invalid assignment target.");
        }

        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.and()?;

        while self.match_(&[Or]) {
            let token = self.previous();
            let right = self.and()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;

        while self.match_(&[And]) {
            let token = self.previous();
            let right = self.equality()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
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
            Ok(Expr::literal(Value::Bool(false)))
        } else if self.match_(&[True]) {
            Ok(Expr::literal(Value::Bool(true)))
        } else if self.match_(&[Nil]) {
            Ok(Expr::literal(Value::Nil))
        } else if self.match_(&[Number, String]) {
            Ok(Expr::literal(self.previous().literal.ok_or_else(|| {
                self.error(self.peek(), "Missing literal")
            })?))
        } else if self.match_(&[Identifier]) {
            Ok(Expr::variable(self.previous()))
        } else if self.match_(&[LeftParen]) {
            let expr = self.expression()?;
            self.consume(RightParen, "Expect ')' after expression.")?;
            Ok(Expr::grouping(expr))
        } else {
            Err(self.error(self.peek(), "Not a valid expression"))
        }
    }
}
