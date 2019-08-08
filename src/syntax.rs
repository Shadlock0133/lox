use crate::{tokens::{Token, Value}, visitor::{Visitor, Visit}};

macro_rules! expr {
    ( $vis:vis enum $name: ident
        { $( $variant:ident ( $( $types:ty ),* $(,)? ) ,)* }
    ) => {
        $(
            #[derive(Debug)]
            $vis struct $variant( $(pub $types),* );
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
                    $( Expr::$variant(e) => {
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
// (outside enum) struct Name(Field1, Field2);
expr!{
    pub enum Expr {
        Binary(Box<Expr>, Token, Box<Expr>),
        Grouping(Box<Expr>),
        Literal(Value),
        Unary(Token, Box<Expr>),
    }
}

impl Expr {
    pub fn binary(left: Expr, op: Token, right: Expr) -> Self {
        Expr::Binary(Binary(Box::new(left), op, Box::new(right)))
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