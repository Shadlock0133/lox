use crate::{
    tokens::{Token, Value},
    visitor::Visitor,
};

macro_rules! ast_gen {
    ( $vis:vis enum $name:ident
        { $( $variant:ident ( $( $typename:ident : $types:ty ),* $(,)? ) ,)* }
    ) => {
        $(
            #[derive(Debug)]
            pub struct $variant{ $(pub $typename: $types),* }
        )*

        #[derive(Debug)]
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

// This turns tuple-variants of an enum into tuple-structs with the same name as variant
// eg. Name(Field1, Field2), turns into
// (in enum) Name(Name),
// (outside enum, in new module) struct Name(Field1, Field2);
// It also implements Visit trait on sub-structs and main enum
ast_gen! {
    pub enum Expr {
        Binary(op: Token, left: Box<Expr>, right: Box<Expr>),
        Unary(op: Token, right: Box<Expr>),
        Grouping(expr: Box<Expr>),
        Literal(value: Value),
        Variable(name: Token),
    }
}

impl Expr {
    pub fn binary(op: Token, left: Expr, right: Expr) -> Self {
        Expr::Binary(Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    pub fn unary(op: Token, right: Expr) -> Expr {
        Expr::Unary(Unary {
            op,
            right: Box::new(right),
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

    pub fn variable(name: Token) -> Self {
        Expr::Variable(Variable { name })
    }
}

ast_gen! {
    pub enum Stmt {
        Expression(expr: Expr),
        PrintStmt(expr: Expr),
        Var(name: Token, init: Option<Expr>),
    }
}

impl Stmt {
    pub fn expression(expr: Expr) -> Self {
        Stmt::Expression(Expression { expr })
    }

    pub fn print(expr: Expr) -> Self {
        Stmt::PrintStmt(PrintStmt { expr })
    }

    pub fn var(name: Token, init: Option<Expr>) -> Self {
        Stmt::Var(Var { name, init })
    }
}
