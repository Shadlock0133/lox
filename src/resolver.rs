use std::collections::HashMap;

use crate::{
    ast::*,
    errors::{ResolveError, ResolveResult},
    tokens::Token,
};

pub struct Resolver {
    locals: HashMap<Expr, usize>,
    scopes: Vec<HashMap<String, bool>>,
}

impl Resolver {
    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn resolve(&mut self, statements: &mut [Stmt]) {
        for stmt in statements {
            self.visit_stmt(stmt);
        }
    }

    fn resolve_local(&mut self, expr: &Expr, name: &Token) {
        for (i, scope) in self.scopes.iter().rev().enumerate() {
            if scope.contains_key(&name.lexeme) {
                self.locals.insert(expr.clone(), i);
                return;
            }
        }
    }

    fn declare(&mut self, name: &Token) {
        self.scopes
            .last_mut()
            .map(|scope| scope.insert(name.lexeme.to_string(), false));
    }

    fn define(&mut self, name: &Token) {
        self.scopes
            .last_mut()
            .map(|scope| scope.insert(name.lexeme.to_string(), true));
    }

    fn visit_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Block { statements } => {
                self.begin_scope();
                self.resolve(statements);
                self.end_scope();
            }
            Stmt::Var { name, init } => {
                self.declare(name);
                if let Some(init) = init {
                    self.visit_expr(init);
                }
                self.define(name);
            }
            _ => todo!(),
        }
    }

    fn visit_expr(&mut self, expr: &mut Expr) {
        match expr {
            _ => todo!(),
        }
    }
}
