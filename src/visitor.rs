use crate::{syntax::*, tokens::Value};

pub trait Visitor<T, R> {
    fn visit(&mut self, _: &mut T) -> R;
}

pub trait Visit<V: Visitor<Self, R>, R>: Sized {
    fn accept(&mut self, v: &mut V) -> R { 
        v.visit(self)
    }
}

impl<T, R, V> Visit<V, R> for Box<T>
where
    T: Visit<V, R>,
    V: Visitor<Self, R> + Visitor<T, R>,
{
    fn accept(&mut self, f: &mut V) -> R { (**self).accept(f) }
}

pub struct Printer;

impl Visitor<Expr, String> for Printer {
    fn visit(&mut self, t: &mut Expr) -> String {
        match t {
            Expr::Binary(e) => self.visit(e),
            Expr::Grouping(e) => self.visit(e),
            Expr::Literal(e) => self.visit(e),
            Expr::Unary(e) => self.visit(e),
        }
    }
}

impl Visitor<Binary, String> for Printer {
    fn visit(&mut self, t: &mut Binary) -> String {
        format!("({} {} {})", t.1.lexeme, (*t.0).accept(self), (*t.2).accept(self))
    }
}

impl Visitor<Grouping, String> for Printer {
    fn visit(&mut self, t: &mut Grouping) -> String {
        format!("(group {})", (*t.0).accept(self))
    }
}

impl Visitor<Literal, String> for Printer {
    fn visit(&mut self, t: &mut Literal) -> String {
        match t.0.clone() {
            Value::String(s) => format!("{}", s),
            Value::Number(n) => format!("{}", n),
            Value::Bool(b) => format!("{}", b),
            Value::Nil => format!("nil"),
        }
    }
}

impl Visitor<Unary, String> for Printer {
    fn visit(&mut self, t: &mut Unary) -> String {
        format!("({} {})", t.0.lexeme, (*t.1).accept(self))
    }
}