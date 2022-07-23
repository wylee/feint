use std::collections::VecDeque;
use std::io::BufRead;

use num_bigint::BigInt;
use num_traits::Num;

use crate::format::scan_format_string;
use crate::util::{Location, Source, Stack};

use super::keywords::KEYWORDS;
use super::result::{ScanErr, ScanErrKind, ScanResult};
use super::token::{Token, TokenWithLocation};

type NextOption<'a> = Option<(char, Option<&'a char>, Option<&'a char>)>;
type NextTwoOption<'a> = Option<(char, char, Option<&'a char>)>;
type NextThreeOption = Option<(char, char, char)>;

pub struct Scanner<'a, T: BufRead> {
    /// This is the source code that's being scanned. T can be anything
    /// that implements the BufRead trait (e.g., a Cursor wrapping some
    /// text or a BufReader wrapping an open file).
    source: &'a mut Source<T>,
    /// Temporary storage for tokens. This is mainly needed to handle
    /// the complexity of indents, because there are cases where
    /// multiple tokens will need to be emitted.
    queue: VecDeque<TokenWithLocation>,
    /// Keep track of whether we're at the start of a line so indents
    /// can be handled specially.
    indent_level: u8,
    /// Stack to keep track of inline blocks (e.g., `block -> true`
    /// where there's no newline after the `->`).
    inline_scope_stack: Stack<(Token, Location)>,
    /// Opening brackets are pushed and later popped when the closing
    /// bracket is encountered. This gives us a way to verify brackets
    /// are matched and also lets us know when we're inside a group
    /// where leading whitespace can be ignored.
    bracket_stack: Stack<(char, Location)>,
    /// The last token that was popped from the queue.
    previous_token: Token,
}

impl<'a, T: BufRead> Scanner<'a, T> {
    pub fn new(source: &'a mut Source<T>) -> Self {
        Scanner {
            source,
            queue: VecDeque::new(),
            indent_level: 0,
            inline_scope_stack: Stack::new(),
            bracket_stack: Stack::new(),
            previous_token: Token::EndOfStatement,
        }
    }

    fn next_token_from_queue(&mut self) -> ScanResult {
        while self.queue.is_empty() {
            self.add_tokens_to_queue()?;
        }
        let token = self.queue.pop_front().unwrap();
        self.previous_token = token.token.clone();
        Ok(token)
    }

    fn err(&self, kind: ScanErrKind, loc: Location) -> Result<(), ScanErr> {
        Err(ScanErr::new(kind, loc))
    }

    fn add_tokens_to_queue(&mut self) -> Result<(), ScanErr> {
        use ScanErrKind::*;
        use Token::*;
        let start = self.source.loc();

        let token = match self.next_char() {
            Some((c @ ('"' | '\''), _, _)) => match self.read_string(c) {
                (s, true) => Str(s),
                (s, false) => {
                    return self.err(UnterminatedStr(format!("{c}{s}")), start);
                }
            },
            Some(('$', Some('"' | '\''), _)) => {
                let d = self.source.next().unwrap();
                match self.read_string(d) {
                    (s, true) => match scan_format_string(s.as_str()) {
                        Ok(tokens) => FormatStr(tokens),
                        Err(err) => {
                            return self.err(FormatStrErr(err), start);
                        }
                    },
                    (s, false) => {
                        return self.err(UnterminatedStr(format!("${d}{s}")), start);
                    }
                }
            }
            Some(('#', _, _)) => {
                self.consume_comment();
                return Ok(());
            }
            Some((':', _, _)) => Colon,
            Some((',', _, _)) => {
                self.maybe_exit_inline_scope(start, false);
                Comma
            }
            Some(('(', _, _)) => {
                self.bracket_stack.push(('(', start));
                LParen
            }
            Some((c @ ')', _, _)) => {
                self.maybe_exit_inline_scope(start, false);
                self.pop_bracket_and_return_token(c, start, RParen)?
            }
            Some(('[', _, _)) => {
                self.bracket_stack.push(('[', start));
                LBracket
            }
            Some((c @ ']', _, _)) => {
                self.maybe_exit_inline_scope(start, false);
                self.pop_bracket_and_return_token(c, start, RBracket)?
            }
            Some(('<', Some('='), _)) => {
                self.consume_char_and_return_token(LessThanOrEqual)
            }
            Some(('<', Some('-'), _)) => self.consume_char_and_return_token(LoopFeed),
            Some(('<', _, _)) => LessThan,
            Some(('>', Some('='), _)) => {
                self.consume_char_and_return_token(GreaterThanOrEqual)
            }
            Some(('>', _, _)) => GreaterThan,
            Some(('=', Some('='), Some('='))) => {
                self.consume_two_chars_and_return_token(EqualEqualEqual)
            }
            Some(('=', Some('='), _)) => self.consume_char_and_return_token(EqualEqual),
            Some(('=', _, _)) => Equal,
            Some(('&', Some('&'), _)) => self.consume_char_and_return_token(And),
            Some(('&', _, _)) => self.consume_char_and_return_token(Ampersand),
            Some(('|', Some('|'), _)) => self.consume_char_and_return_token(Or),
            Some(('|', _, _)) => self.consume_char_and_return_token(Pipe),
            Some(('*', Some('*'), _)) => self.consume_char_and_return_token(DoubleStar),
            Some(('*', Some('='), _)) => self.consume_char_and_return_token(MulEqual),
            Some(('*', _, _)) => Star,
            Some(('/', Some('='), _)) => self.consume_char_and_return_token(DivEqual),
            Some(('/', Some('/'), _)) => {
                self.consume_char_and_return_token(DoubleSlash)
            }
            Some(('/', _, _)) => Slash,
            Some(('+', Some('='), _)) => self.consume_char_and_return_token(PlusEqual),
            Some(('+', _, _)) => {
                // Collapse contiguous plus signs down to a single +.
                // This is safe because + is effectively a no-op.
                self.consume_contiguous('+');
                Plus
            }
            Some(('-', Some('='), _)) => self.consume_char_and_return_token(MinusEqual),
            Some(('-', Some('>'), _)) => {
                let block_token = self.previous_token.clone();
                let end = Location::new(start.line, start.col + 1);
                self.source.next(); // consume >
                self.consume_whitespace();
                if self.source.peek() == Some(&'#') {
                    self.source.next();
                    self.consume_comment();
                }
                if self.source.peek().is_none() {
                    return self.err(ExpectedBlock, self.source.loc());
                } else if self.next_char_is('\n') {
                    // Block
                    self.add_token_to_queue(ScopeStart, start, Some(end));
                    self.expect_indent()?;
                } else {
                    // Inline block
                    let end = Location::new(start.line, start.col + 1);
                    self.add_token_to_queue(InlineScopeStart, start, Some(end));
                    self.inline_scope_stack.push((block_token, start));
                }
                return Ok(());
            }
            Some(('-', _, _)) => Minus,
            Some(('!', Some('='), _)) => self.consume_char_and_return_token(NotEqual),
            Some(('!', _, _)) => {
                // Collapse contiguous bangs down to a single ! or !!.
                // This is mainly to ensure !!!x is interpreted as
                // !(!!(x)) instead of !!(!(x)).
                let count = self.consume_contiguous('!') + 1;
                match count % 2 {
                    0 => BangBang,
                    1 => Bang,
                    _ => unreachable!(),
                }
            }
            Some(('.', Some('.'), Some('.'))) => {
                self.consume_two_chars_and_return_token(RangeInclusive)
            }
            Some(('.', Some('.'), _)) => self.consume_char_and_return_token(Range),
            Some(('.', _, _)) => Dot,
            Some(('%', _, _)) => Percent,
            Some(('^', _, _)) => Caret,
            Some((c @ '0'..='9', _, _)) => match self.read_number(c) {
                (string, _) if string.contains(".") || string.contains("E") => {
                    let value = string
                        .parse::<f64>()
                        .map_err(|err| ScanErr::new(ParseFloatErr(err), start))?;
                    Float(value)
                }
                (string, radix) => {
                    let value = BigInt::from_str_radix(string.as_str(), radix)
                        .map_err(|err| ScanErr::new(ParseIntErr(err), start))?;
                    Int(value)
                }
            },
            // Identifiers
            // Special case for single underscore placeholder var
            Some(('_', _, _)) => {
                if self.consume_contiguous('_') > 0 {
                    return self.err(
                        UnexpectedChar('_'),
                        Location::new(start.line, start.col + 1),
                    );
                }
                Ident("_".to_owned())
            }
            Some((c @ 'a'..='z', _, _)) => {
                let ident = self.read_ident(c);
                let (prev, next) = (&self.previous_token, self.source.peek());
                if let (EndOfStatement | ScopeStart, Some(':')) = (prev, next) {
                    self.source.next();
                    Label(ident)
                } else {
                    match KEYWORDS.get(ident.as_str()) {
                        Some(token) => {
                            if token == &Else {
                                self.maybe_exit_inline_scope(start, true);
                            }
                            token.clone()
                        }
                        _ => Ident(ident),
                    }
                }
            }
            Some((c @ 'A'..='Z', _, _)) => TypeIdent(self.read_type_ident(c)),
            Some((c @ '@', Some('a'..='z'), _)) => TypeMethodIdent(self.read_ident(c)),
            Some((c @ '$', Some('a'..='z'), _)) => {
                SpecialMethodIdent(self.read_ident(c))
            }
            // Newlines
            Some(('\n', _, _)) => {
                if self.bracket_stack.size() == 0 {
                    self.maybe_exit_inline_scope(start, false);
                    self.maybe_add_end_of_statement_token(start);
                    self.dedent()?;
                } else {
                    self.consume_whitespace();
                }
                return Ok(());
            }
            Some((c, _, _)) if c.is_whitespace() => {
                return self.err(UnexpectedWhitespace, start);
            }
            // Unknown
            Some((c, _, _)) => {
                return self.err(UnexpectedChar(c), start);
            }
            // End of input
            None => {
                if self.bracket_stack.size() == 0 {
                    self.maybe_exit_inline_scope(start, false);
                    self.maybe_add_end_of_statement_token(start);
                    self.set_indent_level(0, Location::new(start.line + 1, 1))?;
                } else if let Some((c, location)) = self.bracket_stack.pop() {
                    return self.err(UnmatchedOpeningBracket(c), location);
                }
                EndOfInput
            }
        };

        self.add_token_to_queue(token, start, None);
        self.consume_whitespace();

        Ok(())
    }

    fn assert_start_of_line(&self, name: &str) {
        assert_eq!(
            self.source.current_char,
            Some('\n'),
            "Method should only be called at the start of a line: {name}",
        );
    }

    /// Get the next indent level. Blank lines, whitespace-only lines,
    /// and comment-only lines are skipped over.
    fn get_next_indent_level(&mut self) -> Result<u8, ScanErr> {
        use ScanErrKind::{InvalidIndent, WhitespaceAfterIndent};
        let next_level = loop {
            let num_spaces = self.consume_contiguous(' ');
            let whitespace_count = self.consume_whitespace();
            if let Some(char) = self.source.peek() {
                if *char == '\n' {
                    // Blank or whitespace-only line; skip it.
                    self.source.next();
                    continue;
                } else if *char == '#' {
                    self.consume_comment();
                    continue;
                }
                if num_spaces % 4 != 0 {
                    let loc = self.source.loc();
                    return Err(ScanErr::new(InvalidIndent(num_spaces), loc));
                }
                if whitespace_count > 0 {
                    let loc = self.source.loc();
                    return Err(ScanErr::new(WhitespaceAfterIndent, loc));
                }
                break num_spaces / 4;
            } else {
                break 0;
            }
        };
        Ok(next_level)
    }

    /// Expect the indent level to increase by one.
    fn expect_indent(&mut self) -> Result<(), ScanErr> {
        self.assert_start_of_line("expect_indent");
        let loc = self.source.loc();
        let loc = Location::new(loc.line + 1, 1);
        let current_level = self.indent_level;
        let expected_level = current_level + 1;
        let new_level = self.get_next_indent_level()?;
        if new_level < expected_level {
            return self.err(ScanErrKind::ExpectedIndentedBlock(expected_level), loc);
        }
        self.set_indent_level(new_level, loc)
    }

    /// Handle dedent after a newline is encountered.
    fn dedent(&mut self) -> Result<(), ScanErr> {
        self.assert_start_of_line("dedent");
        let start = self.source.loc();
        let next_level = self.get_next_indent_level()?;
        if next_level > self.indent_level {
            return self.err(ScanErrKind::UnexpectedIndent(next_level), start);
        }
        self.set_indent_level(next_level, start)
    }

    /// Maybe update the current indent level. If the new indent level
    /// is the same as the current indent level, do nothing. If it has
    /// increased, that signals the start of a block (scopes). If it has
    /// decreased, that signals the end of one or more blocks (scopes).
    fn set_indent_level(
        &mut self,
        indent_level: u8,
        start: Location,
    ) -> Result<(), ScanErr> {
        let mut current_level = self.indent_level;
        if indent_level == current_level {
            // Stayed the same; nothing to do
        } else if indent_level == current_level + 1 {
            // Increased by one level
            self.indent_level = indent_level;
        } else if indent_level < current_level {
            // Decreased by one or more levels
            while current_level > indent_level {
                self.exit_block_scope(Location::new(start.line, 0));
                current_level -= 1;
            }
            self.indent_level = current_level;
        } else {
            // Increased by *more* than one level
            return self.err(ScanErrKind::UnexpectedIndent(indent_level), start);
        }
        Ok(())
    }

    fn exit_block_scope(&mut self, loc: Location) {
        self.add_token_to_queue(Token::ScopeEnd, loc, Some(loc));
        self.add_token_to_queue(Token::EndOfStatement, loc, Some(loc));
    }

    fn exit_inline_scope(&mut self, loc: Location, is_else: bool) {
        while !self.inline_scope_stack.is_empty() {
            let (token, _) = self.inline_scope_stack.pop().unwrap();
            self.add_token_to_queue(Token::InlineScopeEnd, loc, Some(loc));
            if is_else && (token == Token::If || token == Token::Else) {
                break;
            }
        }
    }

    /// The scope for an inline block ends when one of the following
    /// tokens is encountered: comma, closing bracket, newline, end of
    /// input.
    fn maybe_exit_inline_scope(&mut self, loc: Location, is_else: bool) {
        if !self.inline_scope_stack.is_empty() {
            self.exit_inline_scope(loc, is_else);
        }
    }

    fn maybe_add_end_of_statement_token(&mut self, loc: Location) {
        use Token::{
            EndOfStatement, InlineScopeEnd, InlineScopeStart, ScopeEnd, ScopeStart,
        };
        match self.previous_token {
            EndOfStatement | InlineScopeStart | InlineScopeEnd | ScopeStart
            | ScopeEnd => (),
            _ => self.add_token_to_queue(EndOfStatement, loc, Some(loc)),
        };
    }

    fn add_token_to_queue(
        &mut self,
        token: Token,
        start: Location,
        end_option: Option<Location>,
    ) {
        let end = match end_option {
            Some(end) => end,
            None => Location::new(
                self.source.line_no,
                if self.source.col == 0 { 0 } else { self.source.col - 1 },
            ),
        };
        let token_with_location = TokenWithLocation::new(token, start, end);
        self.queue.push_back(token_with_location);
    }

    /// Consume the next character and return the specified token.
    fn consume_char_and_return_token(&mut self, token: Token) -> Token {
        self.source.next();
        token
    }

    /// Consume the next two characters and return the specified token.
    fn consume_two_chars_and_return_token(&mut self, token: Token) -> Token {
        self.source.next();
        self.source.next();
        token
    }

    /// Check the specified closing bracket to ensure the last opening
    /// bracket matches. If it does, the specified token is returned.
    #[rustfmt::skip]
    fn pop_bracket_and_return_token(
        &mut self,
        closing_bracket: char,
        location: Location,
        token: Token,
    ) -> Result<Token, ScanErr> {
        match (self.bracket_stack.pop(), closing_bracket) {
            | (Some(('(', _)), ')')
            | (Some(('[', _)), ']')
            => {
                Ok(token)
            }
            _ => Err(ScanErr::new(
                ScanErrKind::UnmatchedClosingBracket(closing_bracket),
                location,
            ))
        }
    }

    /// Consume and return the next character. The following two
    /// characters are included as well for easy peeking.
    fn next_char(&mut self) -> NextOption {
        match self.source.next() {
            Some(c) => {
                let (d, e) = self.source.peek_2();
                Some((c, d, e))
            }
            _ => None,
        }
    }

    /// Consume the next character if it's equal to the specified
    /// character.
    fn next_char_is(&mut self, char: char) -> bool {
        match self.source.peek() {
            Some(next_char) => {
                if *next_char == char {
                    self.source.next();
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// Consume and return the next character if it matches the
    /// specified condition.
    fn next_char_if(&mut self, func: impl FnOnce(&char) -> bool) -> NextOption {
        if let Some(c) = self.source.peek() {
            if func(c) {
                let c = self.source.next().unwrap();
                let (d, e) = self.source.peek_2();
                return Some((c, d, e));
            }
        }
        None
    }

    /// Consume and return the next two characters if the next two
    /// characters match their respective conditions.
    fn next_two_chars_if(
        &mut self,
        c_func: impl FnOnce(&char) -> bool,
        d_func: impl FnOnce(&char) -> bool,
    ) -> NextTwoOption {
        match self.source.peek_2() {
            (Some(c), Some(d)) => match c_func(c) && d_func(d) {
                true => {
                    let c = self.source.next().unwrap();
                    let d = self.source.next().unwrap();
                    let e = self.source.peek();
                    Some((c, d, e))
                }
                false => None,
            },
            _ => None,
        }
    }

    /// Consume and return the next three characters if the next three
    /// characters match their respective conditions.
    fn next_three_chars_if(
        &mut self,
        c_func: impl FnOnce(&char) -> bool,
        d_func: impl FnOnce(&char) -> bool,
        e_func: impl FnOnce(&char) -> bool,
    ) -> NextThreeOption {
        let (c, d, e) = self.source.peek_3();
        match (c, d, e) {
            (Some(c), Some(d), Some(e)) => match c_func(c) && d_func(d) && e_func(e) {
                true => {
                    let c = self.source.next().unwrap();
                    let d = self.source.next().unwrap();
                    let e = self.source.next().unwrap();
                    Some((c, d, e))
                }
                false => None,
            },
            _ => None,
        }
    }

    /// Consume contiguous whitespace up to the end of the line. Return
    /// the number of whitespace characters consumed.
    fn consume_whitespace(&mut self) -> u8 {
        let mut count = 0;
        loop {
            match self.next_char_if(|&c| c != '\n' && c.is_whitespace()) {
                Some(_) => count += 1,
                None => break count,
            }
        }
    }

    /// Consume comment characters up to newline.
    fn consume_comment(&mut self) {
        while self.next_char_if(|&c| c != '\n').is_some() {}
    }

    /// Consume contiguous chars and return count.
    fn consume_contiguous(&mut self, char: char) -> u8 {
        let mut count = 0;
        while self.next_char_is(char) {
            count += 1;
        }
        count
    }

    /// Read a number. Base 2, 8, 10, and 16 ints are supported as well
    /// as base 10 floats.
    fn read_number(&mut self, first_digit: char) -> (String, u32) {
        let mut string = String::new();

        let radix: u32 = if first_digit == '0' {
            match self.source.peek() {
                Some('b') | Some('B') => 2,
                Some('o') | Some('O') => 8,
                Some('x') | Some('X') => 16,
                Some(t) if t.is_ascii_alphabetic() => {
                    panic!("Unsupported numeric type: {}", t);
                }
                _ => 10,
            }
        } else {
            10
        };

        if radix == 10 {
            string.push(first_digit);
        } else {
            // Skip leading zero *and* type char.
            self.source.next();
        }

        string.push_str(self.collect_digits(radix).as_str());

        if radix == 10 {
            match self.next_two_chars_if(|&c| c == '.', |&d| d.is_digit(radix)) {
                // If the number is followed by a dot and at least one
                // digit consume the dot, the digit, and any following
                // digits.
                Some((dot, digit, _)) => {
                    string.push(dot);
                    string.push(digit);
                    string.push_str(self.collect_digits(radix).as_str());
                }
                _ => (),
            }
            // Handle E notation *without* sign.
            match self
                .next_two_chars_if(|&c| c == 'e' || c == 'E', |&e| e.is_digit(radix))
            {
                Some((_, digit, _)) => {
                    string.push('E');
                    string.push('+');
                    string.push(digit);
                    string.push_str(self.collect_digits(radix).as_str());
                }
                _ => (),
            }
            // Handle E notation *with* sign.
            match self.next_three_chars_if(
                |&c| c == 'e' || c == 'E',
                |&d| d == '+' || d == '-',
                |&e| e.is_digit(radix),
            ) {
                Some((_, sign, digit)) => {
                    string.push('E');
                    string.push(sign);
                    string.push(digit);
                    string.push_str(self.collect_digits(radix).as_str());
                }
                _ => (),
            }
        }

        (string, radix)
    }

    fn collect_digits(&mut self, radix: u32) -> String {
        let mut digits = String::new();
        loop {
            match self.next_char_if(|&c| c.is_digit(radix)) {
                Some((digit, _, _)) => digits.push(digit),
                None => {
                    match self.next_two_chars_if(|&c| c == '_', |&d| d.is_digit(radix))
                    {
                        Some((_, digit, _)) => digits.push(digit),
                        None => break digits,
                    }
                }
            }
        }
    }

    /// Read characters inside quotes into a new string. Note that the
    /// returned string does *not* include the opening and closing quote
    /// characters. Quotes can be embedded in a string by backslash-
    /// escaping them.
    fn read_string(&mut self, quote: char) -> (String, bool) {
        let mut string = String::new();
        loop {
            if let Some((_, d, _)) = self.next_two_chars_if(|c| c == &'\\', |_d| true) {
                // Handle chars escaped by a preceding \.
                // TODO: Handle \o, \u, \x, etc
                match d {
                    // Skip newline when preceded by \ at end of
                    // line. Note that this is the case where an
                    // actual newline is embedded in a multiline
                    // string and not the case where the string
                    // "\n" was typed out. The "\n" case is handled
                    // below.
                    '\n' => (),

                    'a' => string.push('\x07'), // bell
                    'b' => string.push('\x08'), // backspace
                    'f' => string.push('\x0c'), // form feed

                    // These next few lines might seem pointless,
                    // but they're replacing the escape sequence in
                    // the source text with the *actual* char in the
                    // Rust string.
                    '0' => string.push('\0'), // null
                    'n' => string.push('\n'), // line feed
                    'r' => string.push('\r'), // carriage return
                    't' => string.push('\t'), // horizontal tab

                    'v' => string.push('\x0b'), // vertical tab

                    '\\' => string.push('\\'),

                    // Unescape escaped single quote. Seems to be
                    // standard (Python and Rust both do it).
                    '\'' => string.push('\''),

                    // This also seems to be a standard.
                    '\"' => string.push('\"'),

                    // Any other escaped char resolves to the
                    // original *escaped* version of itself.
                    other => {
                        string.push('\\');
                        string.push(other);
                    }
                }
            } else {
                match self.source.next() {
                    // Found closing quote; return string.
                    Some(c) if c == quote => break (string, true),
                    // Append current char and continue.
                    Some(c) => string.push(c),
                    // End of input reached without finding closing quote :(
                    None => break (string, false),
                }
            }
        }
    }

    /// Read variable/function identifier.
    ///
    /// Identifiers:
    ///
    /// - start with a lower case ASCII letter (a-z)
    /// - contain lower case ASCII letters, numbers, and underscores
    /// - end with a lower case ASCII letter or number
    ///
    /// NOTE: Identifiers that don't end with a char as noted above will
    ///       cause an error later.
    fn read_ident(&mut self, first_char: char) -> String {
        let mut string = first_char.to_string();
        loop {
            match self
                .next_char_if(|&c| c.is_ascii_lowercase() || c.is_digit(10) || c == '_')
            {
                Some((c, _, _)) => string.push(c),
                None => break string,
            }
        }
    }

    /// Read type identifier.
    ///
    /// Type identifiers:
    ///
    /// - start with an upper case ASCII letter (A-Z)
    /// - contain ASCII letters and numbers
    fn read_type_ident(&mut self, first_char: char) -> String {
        let mut string = first_char.to_string();
        loop {
            match self.next_char_if(|&c| c.is_ascii_alphabetic() || c.is_digit(10)) {
                Some((c, _, _)) => string.push(c),
                None => break string,
            }
        }
    }
}

impl<'a, T: BufRead> Iterator for Scanner<'a, T> {
    type Item = ScanResult;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token_from_queue() {
            Ok(TokenWithLocation { token: Token::EndOfInput, .. }) => None,
            Ok(token) => Some(Ok(token)),
            err => Some(err),
        }
    }
}
