use std::fmt;
use std::str::FromStr;

use num_bigint::BigInt;

use crate::util::{BinaryOperator, UnaryOperator};

/// Program - a list of statements.
#[derive(PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<String> =
            self.statements.iter().map(|statement| format!("{:?}", statement)).collect();
        write!(f, "{}", items.join("\n"))
    }
}

/// Block - a list of statements in a new scope.
#[derive(PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
}

impl Block {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<String> =
            self.statements.iter().map(|statement| format!("{:?}", statement)).collect();
        write!(f, "Block ->\n{}", items.join("\n    "))
    }
}

/// Statement - a logical chunk of code.
#[derive(PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
}

#[derive(PartialEq)]
pub enum StatementKind {
    Print,
    Label(String),
    JumpToLabel(String),
    Expr(Box<Expr>),
}

impl Statement {
    pub fn new(kind: StatementKind) -> Self {
        Self { kind }
    }

    pub fn new_print() -> Self {
        Self::new(StatementKind::Print)
    }

    pub fn new_label<S: Into<String>>(name: S) -> Self {
        Self::new(StatementKind::Label(name.into()))
    }

    pub fn new_jump_to_label<S: Into<String>>(name: S) -> Self {
        Self::new(StatementKind::JumpToLabel(name.into()))
    }

    pub fn new_expr(expr: Expr) -> Self {
        Self::new(StatementKind::Expr(Box::new(expr)))
    }

    pub fn new_nil() -> Self {
        Self::new_expr(Expr::new_literal(Literal::new_nil()))
    }

    pub fn new_string(value: &str) -> Self {
        Self::new_expr(Expr::new_literal(Literal::new_string(value)))
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Statement({:?})", self.kind)
    }
}

impl fmt::Debug for StatementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expr(expr) => write!(f, "Expr({:?})", expr),
            Self::Print => write!(f, "Print"),
            Self::Label(name) => write!(f, "Label: {}", name),
            Self::JumpToLabel(name) => write!(f, "Jump: {}", name),
        }
    }
}

/// Expression - a statement that returns a value.
#[derive(PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
}

#[derive(PartialEq)]
pub enum ExprKind {
    UnaryOp(UnaryOperator, Box<Expr>),
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    Block(Box<Block>),
    Function(String, Box<Block>),
    Literal(Box<Literal>),
    Ident(Box<Ident>),
}

impl Expr {
    pub fn new(kind: ExprKind) -> Self {
        Self { kind }
    }

    pub fn new_block(statements: Vec<Statement>) -> Self {
        Self::new(ExprKind::Block(Box::new(Block::new(statements))))
    }

    pub fn new_unary_op(operator: &str, a: Expr) -> Self {
        let operator = match UnaryOperator::from_str(operator) {
            Ok(op) => op,
            Err(err) => panic!("{}", err),
        };
        Self::new(ExprKind::UnaryOp(operator, Box::new(a)))
    }

    pub fn new_binary_op(a: Expr, operator: &str, b: Expr) -> Self {
        let operator = match BinaryOperator::from_str(operator) {
            Ok(op) => op,
            Err(err) => panic!("{}", err),
        };
        Self::new(ExprKind::BinaryOp(Box::new(a), operator, Box::new(b)))
    }

    pub fn new_ident(ident: Ident) -> Self {
        Self::new(ExprKind::Ident(Box::new(ident)))
    }

    pub fn new_literal(literal: Literal) -> Self {
        Self::new(ExprKind::Literal(Box::new(literal)))
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

impl fmt::Debug for ExprKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnaryOp(op, b) => write!(f, "({:?}{:?})", op, b),
            Self::BinaryOp(a, op, b) => write!(f, "({:?} {:?} {:?})", a, op, b),
            Self::Literal(literal) => write!(f, "{:?}", literal),
            Self::Ident(ident) => write!(f, "{:?}", ident),
            _ => unimplemented!(),
        }
    }
}

/// Literal - a literal value written in the source code, such as 123,
/// 1.23, or "123".
#[derive(PartialEq)]
pub struct Literal {
    pub kind: LiteralKind,
}

#[derive(PartialEq)]
pub enum LiteralKind {
    Nil,
    Bool(bool),
    Float(f64),
    Int(BigInt),
    String(String),
    FormatString(String),
}

impl Literal {
    pub fn new(kind: LiteralKind) -> Self {
        Self { kind }
    }

    pub fn new_nil() -> Self {
        Self::new(LiteralKind::Nil)
    }

    pub fn new_bool(value: bool) -> Self {
        Self::new(LiteralKind::Bool(value))
    }

    pub fn new_float(value: f64) -> Self {
        Self::new(LiteralKind::Float(value))
    }

    pub fn new_int(value: BigInt) -> Self {
        Self::new(LiteralKind::Int(value))
    }

    pub fn new_string<S: Into<String>>(value: S) -> Self {
        Self::new(LiteralKind::String(value.into()))
    }

    pub fn new_format_string<S: Into<String>>(value: S) -> Self {
        Self::new(LiteralKind::FormatString(value.into()))
    }
}

impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

impl fmt::Debug for LiteralKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match self {
            Self::Nil => "nil".to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::FormatString(value) => value.clone(),
        };
        write!(f, "{}", string)
    }
}

/// Identifiers - names for variables, functions, types, and methods
#[derive(PartialEq)]
pub struct Ident {
    pub kind: IdentKind,
}

#[derive(PartialEq)]
pub enum IdentKind {
    Ident(String),
    TypeIdent(String),
}

impl Ident {
    pub fn new(kind: IdentKind) -> Self {
        Self { kind }
    }

    pub fn new_ident(name: String) -> Self {
        Self::new(IdentKind::Ident(name))
    }

    pub fn new_type_ident(name: String) -> Self {
        Self::new(IdentKind::TypeIdent(name))
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

impl fmt::Debug for IdentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Ident(name) => name,
            Self::TypeIdent(name) => name,
        };
        write!(f, "{}", name)
    }
}
