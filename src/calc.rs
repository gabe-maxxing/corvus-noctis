//! `calc`: 64-bit integer calculator for the Corvus shell (CrowC M1 step 1).
//!
//! Grammar (precedence climbing, no heap):
//!   expr   = term   (('+' | '-') term)*
//!   term   = unary  (('*' | '/' | '%') unary)*
//!   unary  = '-' unary | '+' unary | atom
//!   atom   = number | '(' expr ')'
//! All arithmetic is checked: overflow and division by zero are typed errors,
//! never panics. Nesting depth is bounded so hostile input cannot blow the stack.

const MAX_DEPTH: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    Unexpected(usize),
    MissingParen(usize),
    DivisionByZero,
    Overflow,
    TooDeep,
}

impl Error {
    pub fn message(&self) -> &'static str {
        match self {
            Error::Empty => "empty expression",
            Error::Unexpected(_) => "unexpected character",
            Error::MissingParen(_) => "missing )",
            Error::DivisionByZero => "division by zero",
            Error::Overflow => "number too large (64-bit overflow)",
            Error::TooDeep => "expression nested too deeply",
        }
    }

    /// 1-based column in the expression, when the error has one.
    pub fn column(&self) -> Option<usize> {
        match self {
            Error::Unexpected(at) | Error::MissingParen(at) => Some(at + 1),
            _ => None,
        }
    }
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn skip_spaces(&mut self) {
        while self.pos < self.src.len() && (self.src[self.pos] == b' ' || self.src[self.pos] == b'\t') {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_spaces();
        self.src.get(self.pos).copied()
    }

    fn enter(&mut self) -> Result<(), Error> {
        self.depth += 1;
        if self.depth > MAX_DEPTH { Err(Error::TooDeep) } else { Ok(()) }
    }

    fn expr(&mut self) -> Result<i64, Error> {
        self.enter()?;
        let mut value = self.term()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    value = value.checked_add(self.term()?).ok_or(Error::Overflow)?;
                }
                Some(b'-') => {
                    self.pos += 1;
                    value = value.checked_sub(self.term()?).ok_or(Error::Overflow)?;
                }
                _ => break,
            }
        }
        self.depth -= 1;
        Ok(value)
    }

    fn term(&mut self) -> Result<i64, Error> {
        let mut value = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(op @ (b'*' | b'/' | b'%')) => op,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.unary()?;
            value = match op {
                b'*' => value.checked_mul(rhs).ok_or(Error::Overflow)?,
                _ if rhs == 0 => return Err(Error::DivisionByZero),
                b'/' => value.checked_div(rhs).ok_or(Error::Overflow)?,
                _ => value.checked_rem(rhs).ok_or(Error::Overflow)?,
            };
        }
        Ok(value)
    }

    fn unary(&mut self) -> Result<i64, Error> {
        match self.peek() {
            Some(b'-') => {
                self.pos += 1;
                self.enter()?;
                let value = self.unary()?.checked_neg().ok_or(Error::Overflow)?;
                self.depth -= 1;
                Ok(value)
            }
            Some(b'+') => {
                self.pos += 1;
                self.enter()?;
                let value = self.unary()?;
                self.depth -= 1;
                Ok(value)
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) -> Result<i64, Error> {
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                let value = self.expr()?;
                if self.peek() == Some(b')') {
                    self.pos += 1;
                    Ok(value)
                } else {
                    Err(Error::MissingParen(self.pos))
                }
            }
            Some(b'0'..=b'9') => {
                let mut value: i64 = 0;
                while let Some(digit @ b'0'..=b'9') = self.src.get(self.pos).copied() {
                    value = value
                        .checked_mul(10)
                        .and_then(|v| v.checked_add((digit - b'0') as i64))
                        .ok_or(Error::Overflow)?;
                    self.pos += 1;
                }
                Ok(value)
            }
            Some(_) => Err(Error::Unexpected(self.pos)),
            None if self.src.iter().all(|b| *b == b' ' || *b == b'\t') => Err(Error::Empty),
            None => Err(Error::Unexpected(self.pos)),
        }
    }
}

pub fn eval(text: &str) -> Result<i64, Error> {
    let mut parser = Parser { src: text.as_bytes(), pos: 0, depth: 0 };
    let value = parser.expr()?;
    match parser.peek() {
        None => Ok(value),
        Some(_) => Err(Error::Unexpected(parser.pos)),
    }
}

/// Formats an i64 into `buf` without allocating; returns the digits.
pub fn format(value: i64, buf: &mut [u8; 20]) -> &str {
    let mut magnitude = value.unsigned_abs();
    let mut at = buf.len();
    loop {
        at -= 1;
        buf[at] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }
    if value < 0 {
        at -= 1;
        buf[at] = b'-';
    }
    core::str::from_utf8(&buf[at..]).unwrap_or("?")
}
