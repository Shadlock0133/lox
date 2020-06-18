use std::collections::HashMap;

use crate::{
    errors::{ResolveError, ResolveResult},
    ast::*,
    tokens::Token,
};

pub struct VariableResolver {
    locals: HashMap<Expr, usize>,
    scopes: Vec<HashMap<String, bool>>,
}

impl VariableResolver {
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
                self.locals.insert(expr, i);
                return;
            }
        }
    }

    fn declare(&mut self, name: &Token) {
        self.scopes.last_mut().map(|scope| scope.insert(name.lexeme, false));
    }

    fn define(&mut self, name: &Token) {
        self.scopes.last_mut().map(|scope| scope.insert(name.lexeme, true));
    }
}
