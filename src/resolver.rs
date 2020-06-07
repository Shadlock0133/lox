use crate::{
    errors::{ResolveError, ResolveResult},
    syntax::*,
    tokens::Token,
    visitor::Visitor,
    impl_visitor,
};
use std::collections::HashMap;

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
            self.visit(stmt);
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

impl ExprVisitor<ResolveResult> for VariableResolver {}
impl StmtVisitor<ResolveResult> for VariableResolver {}

// impl_visitor!{ for VariableResolver, (&mut self, t: Block) -> ResolveResult {
//     self.begin_scope();
//     self.resolve(&mut t.statements);
//     self.end_scope();
//     Ok(())
// }}

// impl_visitor!{ for VariableResolver, (&mut self, t: Var) -> ResolveResult {
//     self.declare(&t.name);
//     t.init.as_mut().map(|init| self.visit(init));
//     self.define(&t.name);
//     Ok(())
// }}

// impl_visitor!{ for VariableResolver, (&mut self, t: Variable) -> ResolveResult {
//     if self.scopes.last().map(|scope| !scope.get(&t.name.lexeme).unwrap_or(&false)).unwrap_or(false) {
//         return Err(ResolveError);
//     }
//     self.resolve_local(&Expr::Variable(t), &t.name);
//     Ok(())
// }}