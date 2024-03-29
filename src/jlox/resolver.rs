use std::{
    collections::{hash_map::Entry, HashMap},
    mem::replace,
};

use super::{
    ast::*,
    errors::{ResolveError, ResolveResult},
    tokens::Token,
};

#[derive(Clone, Copy, Debug)]
enum FunctionType {
    None,
    Function,
    Initializer,
    Method,
}

#[derive(Clone, Copy, Debug)]
enum ClassType {
    None,
    Class,
    Subclass,
}

#[derive(Debug)]
pub struct Resolver<'a> {
    locals: &'a mut HashMap<Expr, usize>,
    scopes: Vec<HashMap<String, bool>>,
    current_function_type: FunctionType,
    current_class_type: ClassType,
}

impl<'a> Resolver<'a> {
    pub fn new(locals: &'a mut HashMap<Expr, usize>) -> Self {
        Self {
            locals,
            scopes: vec![],
            current_function_type: FunctionType::None,
            current_class_type: ClassType::None,
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
        let enclosing = replace(&mut self.current_function_type, typ);

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
                        Some(name),
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
                            Some(name),
                            "Double define.",
                        ));
                    }
                    occupied.insert(true);
                }
                Entry::Vacant(_) => {
                    return Err(ResolveError::new(
                        Some(name),
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
            Stmt::Class {
                name,
                superclass,
                methods,
            } => {
                let enclosing =
                    replace(&mut self.current_class_type, ClassType::Class);

                self.declare(name)?;
                self.define(name)?;

                if let Some(superclass) = superclass {
                    if superclass.lexeme == name.lexeme {
                        return Err(ResolveError::new(
                            Some(superclass),
                            "A class can't inherit from itself.",
                        ));
                    }

                    self.current_class_type = ClassType::Subclass;
                    self.visit_expr(&Expr::variable(superclass.clone()))?;

                    self.begin_scope();
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert("super".to_owned(), true);
                }

                self.begin_scope();
                self.scopes.last_mut().unwrap().insert("this".into(), true);

                for method in methods {
                    let typ = if method.name.lexeme == "init" {
                        FunctionType::Initializer
                    } else {
                        FunctionType::Method
                    };
                    self.resolve_function(method, typ)?;
                }

                self.end_scope();
                if superclass.is_some() {
                    self.end_scope();
                }

                self.current_class_type = enclosing;
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
                        Some(keyword),
                        "Can't return from top-level code.",
                    ));
                }
                if let Some(value) = value {
                    if matches!(
                        self.current_function_type,
                        FunctionType::Initializer
                    ) {
                        return Err(ResolveError::new(
                            Some(keyword),
                            "Can't return a value from an initializer.",
                        ));
                    }
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
            Expr::Super { keyword, .. } => match self.current_class_type {
                ClassType::None => {
                    return Err(ResolveError::new(
                        Some(keyword),
                        "Can't use 'super' outside of a class.",
                    ))
                }
                ClassType::Class => {
                    return Err(ResolveError::new(
                        Some(keyword),
                        "Can't use 'super' in a class with no superclass.",
                    ))
                }
                ClassType::Subclass => self.resolve_local(expr, keyword),
            },
            Expr::This { keyword } => {
                if matches!(self.current_class_type, ClassType::None) {
                    return Err(ResolveError::new(
                        Some(keyword),
                        "Can't use 'this' outside of a class.",
                    ));
                }
                self.resolve_local(expr, keyword)
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
                        Some(name),
                        "Can't read local variable in its own initializer.",
                    ));
                }
                self.resolve_local(expr, name)
            }
        }
        Ok(())
    }
}
