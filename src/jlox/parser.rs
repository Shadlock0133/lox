use super::{
    ast::*,
    errors::{ParseError, ParseResult},
    tokens::{
        Token,
        TokenType::{self, *},
    },
    types::Value,
};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
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

    fn match_(&mut self, types: &[TokenType]) -> bool {
        for type_ in types {
            if self.check(*type_) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn error<S: Into<std::string::String>>(
        &mut self,
        token: Token,
        message: S,
    ) -> ParseError {
        ParseError::new(Some(token), message.into())
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
        let decl = if self.match_(&[Class]) {
            self.class()
        } else if self.match_(&[Fun]) {
            self.function("function").map(Stmt::Function)
        } else if self.match_(&[Var]) {
            self.var_declaration()
        } else {
            self.statement()
        };

        if decl.is_err() {
            self.synchronize()
        }
        decl
    }

    fn class(&mut self) -> ParseResult<Stmt> {
        let name = self.consume(Identifier, "Expect class name.")?;

        let superclass = if self.match_(&[Less]) {
            self.consume(Identifier, "Expect superclass name.")?;
            Some(self.previous())
        } else {
            None
        };

        self.consume(LeftBrace, "Expect '{' before class body.")?;

        let mut methods = vec![];
        while !self.check(RightBrace) && !self.is_at_end() {
            methods.push(self.function("method")?);
        }

        self.consume(RightBrace, "Expect '}' after class body.")?;
        Ok(Stmt::class(name, superclass, methods))
    }

    fn function(&mut self, kind: &str) -> ParseResult<Function> {
        let name =
            self.consume(Identifier, format!("Expect {} name.", kind))?;
        self.consume(LeftParen, format!("Expect '(' after {} name.", kind))?;

        let mut params = Vec::new();
        if !self.check(RightParen) {
            loop {
                if params.len() >= 255 {
                    return Err(self.error(
                        self.peek(),
                        "Can't have more than 255 parameters.",
                    ));
                }
                params
                    .push(self.consume(Identifier, "Expect parameter name.")?);
                if !self.match_(&[Comma]) {
                    break;
                }
            }
        }
        self.consume(RightParen, "Expect ')' after parameters.")?;

        self.consume(LeftBrace, format!("Expect '{{' before {} body.", kind))?;
        let body = self.block()?;
        Ok(Function { name, params, body })
    }

    fn var_declaration(&mut self) -> ParseResult<Stmt> {
        let name = self.consume(Identifier, "Expect variable name.")?;
        let init = if self.match_(&[Equal]) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(Semicolon, "Expect ';' after variable statement.")?;
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
            self.block().map(Stmt::block)
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
        self.consume(Semicolon, "Expect ';' after value.")?;
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
        self.consume(Semicolon, "Expect ';' after value.")?;
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

            if let Expr::Variable { name } = expr {
                return Ok(Expr::assign(name, value));
            } else if let Expr::Get { object, name } = expr {
                return Ok(Expr::set(*object, name, value));
            }
            return Err(self.error(equals, "Invalid assignment target."));
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
            } else if self.match_(&[Dot]) {
                let name = self
                    .consume(Identifier, "Expect property name after '.'.")?;
                expr = Expr::get(expr, name);
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
                arguments.push(self.expression()?);
                if arguments.len() >= 255 {
                    return Err(self.error(
                        self.peek(),
                        "Can't have more than 255 arguments.",
                    ));
                }
                if !self.match_(&[Comma]) {
                    break;
                }
            }
        }

        let right_paren =
            self.consume(RightParen, "Expect ')' after arguments.")?;

        Ok(Expr::call(callee, right_paren, arguments))
    }

    fn primary(&mut self) -> ParseResult<Expr> {
        let expr =
            if self.match_(&[False]) {
                Expr::literal(Value::Bool(false))
            } else if self.match_(&[True]) {
                Expr::literal(Value::Bool(true))
            } else if self.match_(&[Nil]) {
                Expr::literal(Value::Nil)
            } else if self.match_(&[Number, String]) {
                Expr::literal(self.previous().literal.ok_or_else(|| {
                    self.error(self.peek(), "Missing literal.")
                })?)
            } else if self.match_(&[Super]) {
                let keyword = self.previous();
                self.consume(Dot, "Expect '.' after 'super'.")?;
                let method =
                    self.consume(Identifier, "Expect superclass method name.")?;
                Expr::super_(keyword, method)
            } else if self.match_(&[This]) {
                Expr::this(self.previous())
            } else if self.match_(&[Identifier]) {
                Expr::variable(self.previous())
            } else if self.match_(&[LeftParen]) {
                let expr = self.expression()?;
                self.consume(RightParen, "Expect ')' after expression.")?;
                Expr::grouping(expr)
            } else {
                return Err(self.error(self.peek(), "Expect expression."));
            };
        Ok(expr)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{tokenizer::Tokenizer, tokens::TokenType},
        *,
    };

    #[test]
    fn test() {
        let expr = |x: &str| {
            Parser::new(
                Tokenizer::new(x.into())
                    .filter(|t| {
                        t.as_ref().map(|t| !t.can_skip()).unwrap_or(true)
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
            )
            .expression_statement()
            .unwrap()
        };

        macro_rules! test_expr {
            ($x:expr, $p:pat) => {
                assert!(matches!(expr($x), $p), "{:?}", expr("1 + 2"));
            };
        }

        test_expr!(
            "1 + 2;",
            Stmt::Expression {
                expr: Expr::Binary {
                    op: Token {
                        type_: TokenType::Plus,
                        ..
                    },
                    ..
                }
            }
        );
    }
}
