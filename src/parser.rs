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

type ParseResult<T> = Result<T, ParseError>;

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
    ) -> ParseResult<Token> {
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

    pub fn parse(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        Ok(statements)
    }

    fn declaration(&mut self) -> ParseResult<Stmt> {
        #[allow(clippy::redundant_closure_call)]
        (|| {
            if self.match_(&[Fun]) {
                self.function("function")
            } else if self.match_(&[Var]) {
                self.var_declaration()
            } else {
                self.statement()
            }
        })()
        .map_err(|x| {
            self.synchronize();
            x
        })
    }

    fn function(&mut self, kind: &str) -> ParseResult<Stmt> {
        let name = self.consume(Identifier, format!("Expect {} name.", kind))?;
        self.consume(LeftParen, format!("Expect '(' after {} name.", kind))?;

        let mut params = Vec::new();
        if !self.check(RightParen) {
            loop {
                if params.len() >= 255 {
                    self.error(self.peek(), "Cannot have more than 255 parameters.");
                }
                params.push(self.consume(Identifier, "Expect parameter name.")?);
                if !self.match_(&[Comma]) {
                    break;
                }
            }
        }
        self.consume(RightParen, "Expect ')' after parameters.")?;

        self.consume(LeftBrace, format!("Expect '{{' before {} body.", kind))?;
        let body = self.block()?;
        Ok(Stmt::function(name, params, body))
    }

    fn var_declaration(&mut self) -> ParseResult<Stmt> {
        let name = self.consume(Identifier, "Expect variable name")?;
        let init = if self.match_(&[Equal]) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(Semicolon, "Expect ';' after variable statement")?;
        Ok(Stmt::var(name, init))
    }

    fn statement(&mut self) -> ParseResult<Stmt> {
        if self.match_(&[For]) {
            self.for_statement()
        } else if self.match_(&[If]) {
            self.if_statement()
        } else if self.match_(&[Print]) {
            self.print_statement()
        } else if self.match_(&[Return]) {
            self.return_statement()
        } else if self.match_(&[While]) {
            self.while_statement()
        } else if self.match_(&[LeftBrace]) {
            Ok(Stmt::block(self.block()?))
        } else {
            self.expression_statement()
        }
    }

    fn for_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(LeftParen, "Expect '(' after for.")?;

        let initializer = if self.match_(&[Semicolon]) {
            None
        } else if self.match_(&[Var]) {
            Some(self.var_declaration()?)
        } else {
            Some(self.expression_statement()?)
        };

        let condition = if !self.check(Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(Semicolon, "Expect ';' after loop condition.")?;

        let increment = if !self.check(RightParen) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(RightParen, "Expect ')' after for clauses.")?;

        let mut body = self.statement()?;

        if let Some(inc) = increment {
            body = Stmt::block(vec![body, Stmt::expression(inc)]);
        }

        body = Stmt::while_(
            condition.unwrap_or_else(|| Expr::literal(Value::Bool(true))),
            body,
        );

        if let Some(init) = initializer {
            body = Stmt::block(vec![init, body]);
        }

        Ok(body)
    }

    fn if_statement(&mut self) -> ParseResult<Stmt> {
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

    fn print_statement(&mut self) -> ParseResult<Stmt> {
        let expr = self.expression()?;
        self.consume(Semicolon, "Expect ';' after value")?;
        Ok(Stmt::print(expr))
    }

    fn return_statement(&mut self) -> ParseResult<Stmt> {
        let keyword = self.previous();
        let value = if !self.check(Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(Semicolon, "Expect ';' after return value.")?;
        Ok(Stmt::return_(keyword, value))
    }

    fn while_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(LeftParen, "Expect '(' after while.")?;
        let condition = self.expression()?;
        self.consume(RightParen, "Expect ')' after while condition.")?;

        let body = self.statement()?;

        Ok(Stmt::while_(condition, body))
    }

    fn block(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut statements = Vec::new();

        while !self.check(RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        self.consume(RightBrace, "Expect '}' after block.")?;

        Ok(statements)
    }

    fn expression_statement(&mut self) -> ParseResult<Stmt> {
        let expr = self.expression()?;
        self.consume(Semicolon, "Expect ';' after value")?;
        Ok(Stmt::expression(expr))
    }

    fn expression(&mut self) -> ParseResult<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> ParseResult<Expr> {
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

    fn or(&mut self) -> ParseResult<Expr> {
        let mut expr = self.and()?;

        while self.match_(&[Or]) {
            let token = self.previous();
            let right = self.and()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn and(&mut self) -> ParseResult<Expr> {
        let mut expr = self.equality()?;

        while self.match_(&[And]) {
            let token = self.previous();
            let right = self.equality()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn equality(&mut self) -> ParseResult<Expr> {
        let mut expr = self.comparison()?;

        while self.match_(&[BangEqual, EqualEqual]) {
            let token = self.previous();
            let right = self.comparison()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> ParseResult<Expr> {
        let mut expr = self.addition()?;

        while self.match_(&[Greater, GreaterEqual, Less, LessEqual]) {
            let token = self.previous();
            let right = self.addition()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn addition(&mut self) -> ParseResult<Expr> {
        let mut expr = self.multiplication()?;

        while self.match_(&[Minus, Plus]) {
            let token = self.previous();
            let right = self.multiplication()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn multiplication(&mut self) -> ParseResult<Expr> {
        let mut expr = self.unary()?;

        while self.match_(&[Slash, Star]) {
            let token = self.previous();
            let right = self.unary()?;
            expr = Expr::binary(token, expr, right);
        }

        Ok(expr)
    }

    fn unary(&mut self) -> ParseResult<Expr> {
        if self.match_(&[Bang, Minus]) {
            let token = self.previous();
            let right = self.unary()?;
            Ok(Expr::unary(token, right))
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> ParseResult<Expr> {
        let mut expr = self.primary()?;

        loop {
            if self.match_(&[LeftParen]) {
                expr = self.finish_call(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> ParseResult<Expr> {
        let mut arguments = Vec::new();

        if !self.check(RightParen) {
            loop {
                if arguments.len() >= 255 {
                    self.error(self.peek(), "Cannot have more than 255 arguments");
                }
                arguments.push(self.expression()?);
                if !self.match_(&[Comma]) {
                    break;
                }
            }
        }

        let right_paren = self.consume(RightParen, "Expect ')' after arguments")?;

        Ok(Expr::call(callee, right_paren, arguments))
    }

    fn primary(&mut self) -> ParseResult<Expr> {
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
