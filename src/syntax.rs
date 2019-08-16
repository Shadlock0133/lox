use crate::{
    tokens::{Token, Value},
    visitor::Visitor,
};

macro_rules! ast_gen {
    ( $vis:vis enum $name:ident
        { $( $variant:ident ( $( $typename:ident : $types:ty ),* $(,)? ) ,)* }
    ) => {
        $(
            #[derive(Debug, Clone)]
            pub struct $variant{ $(pub $typename: $types),* }
        )*

        #[derive(Debug, Clone)]
        $vis enum $name { $($variant($variant)),* }

        impl<R, V> Visitor<$name, R> for V
        where
            $(V: Visitor<$variant, R>),*
        {
            fn visit(&mut self, t: &mut $name) -> R {
                match t {
                    $($name::$variant(inner) => self.visit(inner)),*
                }
            }
        }
    };
}

// This turns struct-variants of an enum into structs with the same name as variant
// eg. Name(name1: Field1, name2: Field2), turns into
// (in enum) Name(Name),
// (outside enum, in new module) struct Name { name1: Field1, name2: Field2 }
ast_gen! {
    pub enum Expr {
        Assign(name: Token, value: Box<Expr>),
        Binary(op: Token, left: Box<Expr>, right: Box<Expr>),
        Call(callee: Box<Expr>, right_paren: Token, arguments: Vec<Expr>),
        Grouping(expr: Box<Expr>),
        Literal(value: Value),
        Unary(op: Token, right: Box<Expr>),
        Variable(name: Token),
    }
}

impl Expr {
    pub fn assign(name: Token, value: Expr) -> Self {
        Expr::Assign(Assign {
            name,
            value: Box::new(value),
        })
    }

    pub fn binary(op: Token, left: Expr, right: Expr) -> Self {
        Expr::Binary(Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    pub fn call(callee: Expr, right_paren: Token, arguments: Vec<Expr>) -> Expr {
        Expr::Call(Call {
            callee: Box::new(callee),
            right_paren,
            arguments,
        })
    }

    pub fn grouping(expr: Expr) -> Self {
        Expr::Grouping(Grouping {
            expr: Box::new(expr),
        })
    }

    pub fn literal(value: Value) -> Self {
        Expr::Literal(Literal { value })
    }

    pub fn unary(op: Token, right: Expr) -> Expr {
        Expr::Unary(Unary {
            op,
            right: Box::new(right),
        })
    }

    pub fn variable(name: Token) -> Self {
        Expr::Variable(Variable { name })
    }
}

ast_gen! {
    pub enum Stmt {
        Block(statements: Vec<Stmt>),
        Expression(expr: Expr),
        Function(name: Token, params: Vec<Token>, body: Vec<Stmt>),
        If(condition: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>>),
        PrintStmt(expr: Expr),
        Var(name: Token, init: Option<Expr>),
        While(condition: Expr, body: Box<Stmt>),
    }
}

impl Stmt {
    pub fn block(statements: Vec<Stmt>) -> Self {
        Stmt::Block(Block { statements })
    }

    pub fn expression(expr: Expr) -> Self {
        Stmt::Expression(Expression { expr })
    }

    pub fn function(name: Token, params: Vec<Token>, body: Vec<Stmt>) -> Self {
        Stmt::Function(Function{ name, params, body })
    }

    pub fn if_(condition: Expr, then_branch: Stmt, else_branch: Option<Stmt>) -> Self {
        Stmt::If(If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        })
    }

    pub fn print(expr: Expr) -> Self {
        Stmt::PrintStmt(PrintStmt { expr })
    }

    pub fn var(name: Token, init: Option<Expr>) -> Self {
        Stmt::Var(Var { name, init })
    }

    pub fn while_(condition: Expr, body: Stmt) -> Self {
        Stmt::While(While {
            condition,
            body: Box::new(body),
        })
    }
}
