pub use keywords::KEYWORDS;
pub use result::{ScanErr, ScanErrKind, ScanResult};
pub use scanner::Scanner;
pub use token::{Token, TokenWithLocation};

mod keywords;
mod result;
mod scanner;
mod token;

#[cfg(test)]
mod tests;
