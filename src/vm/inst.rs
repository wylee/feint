use crate::util::{BinaryOperator, UnaryOperator};

pub type Chunk = Vec<Inst>;

#[derive(Debug, PartialEq)]
pub enum Inst {
    NoOp,
    Push(usize),
    Pop,
    Jump(usize),        // Jump unconditionally
    JumpIfTrue(usize),  // Jump if top of stack is true
    JumpIfFalse(usize), // Jump if top of stack is false
    UnaryOp(UnaryOperator),
    BinaryOp(BinaryOperator),
    LoadConst(usize),
    DeclareVar(String),
    AssignVar(String),
    LoadVar(String),
    ScopeStart,
    ScopeEnd(usize),
    Print, // Print value at top of stack
    Return,
    Halt(i32),
}