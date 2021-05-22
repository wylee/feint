use std::fmt;
use std::fmt::Formatter;

use crate::parser::ParseError;
use crate::vm::compiler::CompilationError;

pub type ExecutionResult = Result<VMState, ExecutionError>;

#[derive(Debug)]
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
}

impl ExecutionError {
    pub fn new(kind: ExecutionErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

#[derive(Debug)]
pub enum ExecutionErrorKind {
    GenericError(String),
    NotEnoughValuesOnStack,
    ParserError(ParseError),
    CompilationError(CompilationError),
}

impl fmt::Display for ExecutionErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq)]
pub enum VMState {
    Idle,
    Halted(i32),
}