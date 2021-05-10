use crate::{tokens::Token, types::Value};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Expr {
    Assign {
        name: Token,
        value: Box<Expr>,
    },
    Binary {
        op: Token,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        right_paren: Token,
        arguments: Vec<Expr>,
    },
    Get {
        object: Box<Expr>,
        name: Token,
    },
    Grouping {
        expr: Box<Expr>,
    },
    Literal {
        value: Value,
    },
    Set {
        object: Box<Expr>,
        name: Token,
        value: Box<Expr>,
    },
    This {
        keyword: Token,
    },
    Unary {
        op: Token,
        right: Box<Expr>,
    },
    Variable {
        name: Token,
    },
}

impl Expr {
    pub fn assign(name: Token, value: Expr) -> Self {
        Self::Assign {
            name,
            value: Box::new(value),
        }
    }

    pub fn binary(op: Token, left: Expr, right: Expr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub fn call(
        callee: Expr,
        right_paren: Token,
        arguments: Vec<Expr>,
    ) -> Self {
        Self::Call {
            callee: Box::new(callee),
            right_paren,
            arguments,
        }
    }

    pub fn get(object: Expr, name: Token) -> Self {
        Self::Get {
            object: Box::new(object),
            name,
        }
    }

    pub fn grouping(expr: Expr) -> Self {
        Self::Grouping {
            expr: Box::new(expr),
        }
    }

    pub fn literal(value: Value) -> Self {
        Self::Literal { value }
    }

    pub fn set(object: Expr, name: Token, value: Expr) -> Self {
        Self::Set {
            object: Box::new(object),
            name,
            value: Box::new(value),
        }
    }

    pub fn this(keyword: Token) -> Self {
        Self::This { keyword }
    }

    pub fn unary(op: Token, right: Expr) -> Self {
        Self::Unary {
            op,
            right: Box::new(right),
        }
    }

    pub fn variable(name: Token) -> Self {
        Self::Variable { name }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },
    Class {
        name: Token,
        methods: Vec<Function>,
    },
    Expression {
        expr: Expr,
    },
    Function(Function),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    PrintStmt {
        expr: Expr,
    },
    Return {
        keyword: Token,
        value: Option<Expr>,
    },
    Var {
        name: Token,
        init: Option<Expr>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Function {
    pub name: Token,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
}

impl Stmt {
    pub fn block(statements: Vec<Stmt>) -> Self {
        Self::Block { statements }
    }

    pub fn class(name: Token, methods: Vec<Function>) -> Self {
        Self::Class { name, methods }
    }

    pub fn expression(expr: Expr) -> Self {
        Self::Expression { expr }
    }

    pub fn function(name: Token, params: Vec<Token>, body: Vec<Stmt>) -> Self {
        Self::Function(Function { name, params, body })
    }

    pub fn if_(
        condition: Expr,
        then_branch: Stmt,
        else_branch: Option<Stmt>,
    ) -> Self {
        Self::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        }
    }

    pub fn print(expr: Expr) -> Self {
        Self::PrintStmt { expr }
    }

    pub fn return_(keyword: Token, value: Option<Expr>) -> Self {
        Self::Return { keyword, value }
    }

    pub fn var(name: Token, init: Option<Expr>) -> Self {
        Self::Var { name, init }
    }

    pub fn while_(condition: Expr, body: Stmt) -> Self {
        Self::While {
            condition,
            body: Box::new(body),
        }
    }
}
