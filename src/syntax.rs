use crate::{
    tokens::{Token, Value},
    visitor::{Visit, Visitor},
};

macro_rules! ast_gen {
    ( $vis:vis enum $name:ident in $modname:ident
        { $( $variant:ident ( $( $types:ty ),* $(,)? ) ,)* }
    ) => {
            $(
                #[derive(Debug)]
                pub struct $variant( $(pub $types),* );
                
                impl<V: Visitor<Self, R>, R> Visit<V, R> for $variant {
                    fn accept(&mut self, v: &mut V) -> R { v.visit(self) }
                }
            )*

        #[derive(Debug)]
        $vis enum $name { $($variant($variant)),* }

        impl<V: Visitor<Self, R>, R> Visit<V, R> for $name
        where
            $(V: Visitor<$variant, R>),*
        {
            fn accept(&mut self, f: &mut V) -> R {
                match self {
                    $( $name::$variant(e) => {
                        e.accept(f)
                    } ),*
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
    pub enum Expr in expr {
        Binary(Token, Box<Expr>, Box<Expr>),
        Grouping(Box<Expr>),
        Literal(Value),
        Unary(Token, Box<Expr>),
    }
}

impl Expr {
    pub fn binary(op: Token, left: Expr, right: Expr) -> Self {
        Expr::Binary(Binary(op, Box::new(left), Box::new(right)))
    }

    pub fn grouping(expr: Expr) -> Self {
        Expr::Grouping(Grouping(Box::new(expr)))
    }

    pub fn literal(literal: Value) -> Self {
        Expr::Literal(Literal(literal))
    }

    pub fn unary(token: Token, expr: Expr) -> Expr {
        Expr::Unary(Unary(token, Box::new(expr)))
    }
}

ast_gen! {
    pub enum Stmt in stmt {
        Expression(Expr),
        PrintStmt(Expr),
    }
}
