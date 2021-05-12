use std::fmt;

use super::Location;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    LeftParen,          // (
    RightParen,         // )
    LeftSquareBracket,  // [
    RightSquareBracket, // ]
    LeftAngleBracket,   // <
    RightAngleBracket,  // >

    Colon, // :
    Comma, // ,

    // Fundamental types
    True,
    False,
    Float(String),    // 1.0
    Int(String, u32), // 1, 0b1, 0o1, ox1 (digits, radix)
    String(String),   // "1" (does NOT include quotes)

    // Single-character operators
    Equal,     // =
    Star,      // *
    Slash,     // /
    Plus,      // +
    Minus,     // -
    Bang,      // !
    Dot,       // .
    Percent,   // %
    Caret,     // ^
    Ampersand, // &
    Pipe,      // |

    // Multi-character operators
    EqualEqual,         // ==
    And,                // &&
    Or,                 // ||
    DoubleStar,         // **
    NotEqual,           // !=
    GreaterThanOrEqual, // >=
    LessThanOrEqual,    // <=
    LoopFeed,           // <-
    Range,              // ..
    RangeInclusive,     // ...
    AsBool,             // !! (the boolean evaluation of an object)

    // In-place operators
    // TODO: If reassignment isn't allowed, these don't make sense
    MulEqual,   // *=
    DivEqual,   // /=
    PlusEqual,  // +=
    MinusEqual, // -=

    // Indicates start of function or block/scope
    FuncStart, // ->

    BlockStart, // Start of indented block
    BlockEnd,   // End of indented block

    // Identifiers
    Identifier(String),              // name
    TypeIdentifier(String),          // Name
    TypeMethodIdentifier(String),    // @name (called via type)
    SpecialMethodIdentifier(String), // $name (e.g., $bool on a type)

    EndOfInput,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token {:?}", self)
    }
}

// A token with its start and end locations in the source.
#[derive(Clone, Debug, PartialEq)]
pub struct TokenWithLocation {
    pub token: Token,
    pub start: Location,
    pub end: Location,
}

impl TokenWithLocation {
    pub fn new(token: Token, start: Location, end: Location) -> Self {
        Self { token, start, end }
    }
}

impl fmt::Display for TokenWithLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} -> {}", self.start, self.end, self.token)
    }
}