use std::collections::{hash_map::Entry, HashMap};

use crate::{
    ast::*,
    errors::{ResolveError, ResolveResult},
    tokens::Token,
};

#[derive(Clone, Copy, Debug)]
enum FunctionType {
    None,
    Function,
    Method,
}

#[derive(Debug)]
pub struct Resolver<'a> {
    locals: &'a mut HashMap<Expr, usize>,
    scopes: Vec<HashMap<String, bool>>,
    current_function_type: FunctionType,
}

impl<'a> Resolver<'a> {
    pub fn new(locals: &'a mut HashMap<Expr, usize>) -> Self {
        Self {
            locals,
            scopes: vec![],
            current_function_type: FunctionType::None,
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn resolve(&mut self, statements: &[Stmt]) -> ResolveResult<()> {
        for stmt in statements {
            self.visit_stmt(stmt)?;
        }
        Ok(())
    }

    fn resolve_local(&mut self, expr: &Expr, name: &Token) {
        for (i, scope) in self.scopes.iter().rev().enumerate() {
            if scope.contains_key(&name.lexeme) {
                self.locals.insert(expr.clone(), i);
                return;
            }
        }
    }

    fn resolve_function(
        &mut self,
        function: &Function,
        typ: FunctionType,
    ) -> ResolveResult<()> {
        let enclosing = std::mem::replace(&mut self.current_function_type, typ);

        self.begin_scope();
        for param in &function.params {
            self.declare(param)?;
            self.define(param)?;
        }
        self.resolve(&function.body)?;
        self.end_scope();

        self.current_function_type = enclosing;

        Ok(())
    }

    fn declare(&mut self, name: &Token) -> ResolveResult<()> {
        if let Some(scope) = self.scopes.last_mut() {
            match scope.entry(name.lexeme.clone()) {
                Entry::Occupied(_) => {
                    return Err(ResolveError::new(
                        Some(name.clone()),
                        "Already variable with this name in this scope.",
                    ))
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(false);
                }
            }
        }
        Ok(())
    }

    fn define(&mut self, name: &Token) -> ResolveResult<()> {
        if let Some(scope) = self.scopes.last_mut() {
            match scope.entry(name.lexeme.clone()) {
                Entry::Occupied(mut occupied) => {
                    if *occupied.get() {
                        return Err(ResolveError::new(
                            Some(name.clone()),
                            "Double define.",
                        ));
                    }
                    occupied.insert(true);
                }
                Entry::Vacant(_) => {
                    return Err(ResolveError::new(
                        Some(name.clone()),
                        "Defining undeclared variable.",
                    ))
                }
            }
        }
        Ok(())
    }

    fn visit_stmt(&mut self, stmt: &Stmt) -> ResolveResult<()> {
        match stmt {
            Stmt::Block { statements } => {
                self.begin_scope();
                self.resolve(statements)?;
                self.end_scope();
            }
            Stmt::Class { name, methods } => {
                self.declare(name)?;
                self.define(name)?;
                for method in methods {
                    self.resolve_function(method, FunctionType::Method)?;
                }
            }
            Stmt::Expression { expr } => self.visit_expr(expr)?,
            Stmt::Function(function) => {
                self.declare(&function.name)?;
                self.define(&function.name)?;
                self.begin_scope();
                self.resolve_function(function, FunctionType::Function)?;
                self.end_scope();
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.visit_expr(condition)?;
                self.visit_stmt(then_branch)?;
                if let Some(else_branch) = else_branch {
                    self.visit_stmt(else_branch)?;
                }
            }
            Stmt::PrintStmt { expr } => self.visit_expr(expr)?,
            Stmt::Return { keyword, value } => {
                if matches!(self.current_function_type, FunctionType::None) {
                    return Err(ResolveError::new(
                        Some(keyword.clone()),
                        "Can't return from top-level code.",
                    ));
                }
                if let Some(value) = value {
                    self.visit_expr(value)?;
                }
            }
            Stmt::While { condition, body } => {
                self.visit_expr(condition)?;
                self.visit_stmt(body)?;
            }
            Stmt::Var { name, init } => {
                self.declare(name)?;
                if let Some(init) = init {
                    self.visit_expr(init)?;
                }
                self.define(name)?;
            }
        }
        Ok(())
    }

    fn visit_expr(&mut self, expr: &Expr) -> ResolveResult<()> {
        match expr {
            Expr::Assign { name, value } => {
                self.visit_expr(value)?;
                self.resolve_local(expr, name);
            }
            Expr::Binary { left, right, .. } => {
                self.visit_expr(left)?;
                self.visit_expr(right)?;
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                self.visit_expr(callee)?;
                for argument in arguments {
                    self.visit_expr(argument)?;
                }
            }
            Expr::Get { object, .. } => self.visit_expr(object)?,
            Expr::Grouping { expr } => self.visit_expr(expr)?,
            Expr::Literal { .. } => {}
            Expr::Set { object, value, .. } => {
                self.visit_expr(value)?;
                self.visit_expr(object)?;
            }
            Expr::Unary { right, .. } => self.visit_expr(right)?,
            Expr::Variable { name } => {
                if self
                    .scopes
                    .last()
                    .and_then(|x| x.get(&name.lexeme))
                    .map(|x| !*x)
                    .unwrap_or(false)
                {
                    return Err(ResolveError::new(
                        Some(name.clone()),
                        "Can't read local variable in its own initializer.",
                    ));
                }
                self.resolve_local(expr, name);
            }
        }
        Ok(())
    }
}
