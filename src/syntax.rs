use crate::{
    tokens::{Token, Value},
    visitor::Visitor,
};

macro_rules! ast_gen {
    ( $vis:vis enum $name:ident
        { $( $variant:ident { $( $typename:ident : $types:ty ),* $(,)? } ,)* }
    ) => {
        $(
            #[derive(Debug, Clone, Hash)]
            pub struct $variant{ $(pub $typename: $types),* }
        )*

        #[derive(Debug, Clone, Hash)]
        // #[allow(clippy::large_enum_variant)]
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

        #[test]
        #[ignore]
        #[allow(non_snake_case)]
        fn $name() {
            eprintln!("Size of {}: {}", stringify!($name), std::mem::size_of::<crate::syntax::$name>());
            $( eprintln!("Size of {}::{}: {}", stringify!($name), stringify!($variant), std::mem::size_of::<crate::syntax::$variant>()); )*
        }
    };
}

// This turns struct-variants of an enum into structs with the same name as variant
// eg. Name { name1: Field1, name2: Field2 }, turns into
// (in enum) Name(Name),
// (outside enum) struct Name { name1: Field1, name2: Field2 }
ast_gen! {
    pub enum Expr {
        Assign { name: Token, value: Box<Expr> },
        Binary { op: Token, left: Box<Expr>, right: Box<Expr> },
        Call { callee: Box<Expr>, right_paren: Token, arguments: Vec<Expr> },
        Grouping { expr: Box<Expr> },
        Literal { value: Value },
        Unary { op: Token, right: Box<Expr> },
        Variable { name: Token },
    }
}

pub trait ExprVisitor<R> {
    fn visit_expr(&mut self, expr: &mut Expr) -> R {
        match expr {
            Expr::Assign(e) => self.visit_assign(e),
            Expr::Binary(e) => self.visit_binary(e),
            Expr::Call(e) => self.visit_call(e),
            Expr::Grouping(e) => self.visit_grouping(e),
            Expr::Literal(e) => self.visit_literal(e),
            Expr::Unary(e) => self.visit_unary(e),
            Expr::Variable(e) => self.visit_variable(e),
        }
    }
    fn visit_assign(&mut self, assign: &mut Assign) -> R { unimplemented!() }
    fn visit_binary(&mut self, binary: &mut Binary) -> R { unimplemented!() }
    fn visit_call(&mut self, call: &mut Call) -> R { unimplemented!() }
    fn visit_grouping(&mut self, grouping: &mut Grouping) -> R { unimplemented!() }
    fn visit_literal(&mut self, literal: &mut Literal) -> R { unimplemented!() }
    fn visit_unary(&mut self, unary: &mut Unary) -> R { unimplemented!() }
    fn visit_variable(&mut self, variable: &mut Variable) -> R { unimplemented!() }
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
        Block { statements: Vec<Stmt> },
        Expression { expr: Expr },
        Function { name: Token, params: Vec<Token>, body: Vec<Stmt> },
        If { condition: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
        PrintStmt { expr: Expr },
        Return { keyword: Token, value: Option<Expr> },
        Var { name: Token, init: Option<Expr> },
        While { condition: Expr, body: Box<Stmt> },
    }
}

pub trait StmtVisitor<R> {
    fn visit_stmt(&mut self, stmt: &mut Stmt) -> R {
        match stmt {
            Stmt::Block(s) => self.visit_block(s),
            Stmt::Expression(s) => self.visit_expression(s),
            Stmt::Function(s) => self.visit_function(s),
            Stmt::If(s) => self.visit_if(s),
            Stmt::PrintStmt(s) => self.visit_print_stmt(s),
            Stmt::Return(s) => self.visit_return(s),
            Stmt::Var(s) => self.visit_var(s),
            Stmt::While(s) => self.visit_while(s),
        }
    }
    fn visit_block(&mut self, block: &mut Block) -> R { unimplemented!() }
    fn visit_expression(&mut self, expression: &mut Expression) -> R { unimplemented!() }
    fn visit_function(&mut self, function: &mut Function) -> R { unimplemented!() }
    fn visit_if(&mut self, if_: &mut If) -> R { unimplemented!() }
    fn visit_print_stmt(&mut self, print_stmt: &mut PrintStmt) -> R { unimplemented!() }
    fn visit_return(&mut self, return_: &mut Return) -> R { unimplemented!() }
    fn visit_var(&mut self, var: &mut Var) -> R { unimplemented!() }
    fn visit_while(&mut self, while_: &mut While) -> R { unimplemented!() }
}

impl Stmt {
    pub fn block(statements: Vec<Stmt>) -> Self {
        Stmt::Block(Block { statements })
    }

    pub fn expression(expr: Expr) -> Self {
        Stmt::Expression(Expression { expr })
    }

    pub fn function(name: Token, params: Vec<Token>, body: Vec<Stmt>) -> Self {
        Stmt::Function(Function { name, params, body })
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

    pub fn return_(keyword: Token, value: Option<Expr>) -> Self {
        Stmt::Return(Return { keyword, value })
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
