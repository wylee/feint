pub(crate) use parser::Parser;
pub(crate) use result::{ParseErr, ParseErrKind};

mod parser;
mod precedence;
mod result;

#[cfg(test)]
pub(crate) use result::ParseResult;
