//! CrowC M1-M3: bounded, allocation-free, immediate expressions and statements.
//! Original interpreter: no TempleOS implementation or executable memory.
use crate::{calc, print};
use super::platform::Effect;
const LIMIT: usize = 4096;
const DEPTH: usize = 32;

#[derive(Clone, Copy)]
struct Text { bytes: [u8; 128], len: usize }
impl Text {
    const EMPTY: Self = Self { bytes: [0; 128], len: 0 };
    fn str(&self) -> &str { core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("") }
    fn push(&mut self, b: u8) -> Result<(), Error> {
        if self.len == self.bytes.len() { return Err(Error("string too long")); }
        self.bytes[self.len] = b; self.len += 1; Ok(())
    }
}
#[derive(Clone, Copy)]
enum Value { Int(i64), Str(Text), Void }
impl Value {
    fn int(self) -> Result<i64, Error> { match self { Self::Int(n) => Ok(n), _ => Err(Error("expected integer")) } }
}
#[derive(Clone, Copy)]
struct Variable { name: [u8; 24], len: usize, value: Value }
impl Variable { const EMPTY: Self = Self { name: [0; 24], len: 0, value: Value::Int(0) }; }
#[derive(Debug, PartialEq)]
pub struct Error(pub &'static str);

pub struct Context {
    pub graphics: for<'a> fn(Effect<'a>),
    pub command: fn(&str) -> bool,
    pub ticks: u64,
    pub cancelled: fn() -> bool,
}

pub struct CrowC { vars: [Variable; 64] }
impl CrowC {
    pub const fn new() -> Self { Self { vars: [Variable::EMPTY; 64] } }
    fn get(&self, name: &str) -> Option<Value> {
        self.vars.iter().find(|v| &v.name[..v.len] == name.as_bytes()).map(|v| v.value)
    }
    fn set(&mut self, name: &str, value: Value) -> Result<(), Error> {
        if name.len() > 24 { return Err(Error("variable name too long")); }
        let index = self.vars.iter().position(|v| &v.name[..v.len] == name.as_bytes())
            .or_else(|| self.vars.iter().position(|v| v.len == 0)).ok_or(Error("64 variables already used; Reset; to clear"))?;
        let v = &mut self.vars[index]; v.name[..name.len()].copy_from_slice(name.as_bytes());
        v.len = name.len(); v.value = value; Ok(())
    }
    pub fn accepts(&self, text: &str) -> bool {
        let text = text.trim_start();
        if matches!(text.as_bytes().first(), Some(b'"' | b'0'..=b'9' | b'(' | b'-' | b'+' | b'!')) { return true; }
        let name = text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').next().unwrap_or("");
        !name.is_empty() && (matches!(name, "let" | "if" | "while") || builtin(name).is_some()
            || self.get(name).is_some() || text[name.len()..].trim_start().starts_with('='))
    }
    pub fn eval(&mut self, source: &str, context: Context) -> Result<(), Error> {
        let mut parser = Parser { source, pos: 0, depth: 0, steps: LIMIT, output: 4096, state: self, context };
        while parser.peek().is_some() { parser.statement(true)?; }
        Ok(())
    }
}

pub fn incomplete(source: &str) -> bool {
    let (mut braces, mut quoted, mut escape) = (0i32, false, false);
    for b in source.bytes() {
        if quoted {
            if escape { escape = false; } else if b == b'\\' { escape = true; } else if b == b'"' { quoted = false; }
        } else { match b { b'"' => quoted = true, b'{' => braces += 1, b'}' => braces -= 1, _ => {} } }
    }
    braces > 0 && !quoted
}
fn builtin(name: &str) -> Option<usize> {
    ["Help", "About", "Crow", "Cls", "Print", "Draw", "Vars", "Reset", "Ticks"].iter().position(|n| n.eq_ignore_ascii_case(name))
}

struct Parser<'a> {
    source: &'a str, pos: usize, depth: usize, steps: usize, output: usize,
    state: &'a mut CrowC, context: Context,
}
impl Parser<'_> {
    fn step(&mut self, count: usize) -> Result<(), Error> {
        if (self.context.cancelled)() { return Err(Error("interrupted")); }
        self.steps = self.steps.checked_sub(count).ok_or(Error("step limit reached"))?; Ok(())
    }
    fn enter(&mut self) -> Result<(), Error> {
        self.step(1)?; self.depth += 1;
        if self.depth > DEPTH { Err(Error("nesting too deep")) } else { Ok(()) }
    }
    fn peek(&mut self) -> Option<u8> {
        while self.source.as_bytes().get(self.pos).is_some_and(u8::is_ascii_whitespace) { self.pos += 1; }
        self.source.as_bytes().get(self.pos).copied()
    }
    fn take(&mut self, token: &str) -> bool {
        self.peek();
        if self.source[self.pos..].starts_with(token) { self.pos += token.len(); true } else { false }
    }
    fn need(&mut self, token: &str) -> Result<(), Error> {
        if self.take(token) { Ok(()) } else { Err(Error("unexpected token or missing delimiter")) }
    }
    fn ident(&mut self) -> Option<&str> {
        self.peek(); let start = self.pos;
        if !self.source.as_bytes().get(start).is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_') { return None; }
        while self.source.as_bytes().get(self.pos).is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_') { self.pos += 1; }
        Some(&self.source[start..self.pos])
    }
    fn keyword(&mut self, name: &str) -> bool {
        let before = self.pos;
        if self.ident() == Some(name) { true } else { self.pos = before; false }
    }
    fn statement(&mut self, run: bool) -> Result<(), Error> {
        self.enter()?;
        if self.take(";") { self.depth -= 1; return Ok(()); }
        if self.keyword("if") {
            self.need("(")?; let yes = self.expr(0, run)?.int()? != 0; self.need(")")?;
            self.block(run && yes)?;
            if self.keyword("else") { self.block(run && !yes)?; }
        } else if self.keyword("while") {
            let start = self.pos;
            loop {
                self.pos = start; self.need("(")?;
                let yes = self.expr(0, run)?.int()? != 0; self.need(")")?;
                self.block(run && yes)?;
                if !run || !yes { break; }
            }
        } else {
            let declared = self.keyword("let");
            let before = self.pos;
            let mut name = [0u8; 24]; let mut len = 0;
            if let Some(id) = self.ident() {
                if id.len() <= name.len() { len = id.len(); name[..len].copy_from_slice(id.as_bytes()); }
            }
            self.peek();
            if len > 0 && self.source[self.pos..].starts_with('=') && !self.source[self.pos..].starts_with("==") {
                self.pos += 1;
                let value = self.expr(0, run)?;
                if run { self.state.set(core::str::from_utf8(&name[..len]).unwrap_or(""), value)?; }
            } else {
                if declared { return Err(Error("expected let name = expression")); }
                self.pos = before;
                let value = self.expr(0, run)?;
                if run { self.emit_value(value)?; }
            }
            if !self.take(";") && !matches!(self.peek(), None | Some(b'}')) { return Err(Error("expected ; between statements")); }
        }
        self.depth -= 1; Ok(())
    }
    fn block(&mut self, run: bool) -> Result<(), Error> {
        self.need("{")?;
        while self.peek().is_some() && self.peek() != Some(b'}') { self.statement(run)?; }
        self.need("}")
    }
    fn expr(&mut self, minimum: u8, run: bool) -> Result<Value, Error> {
        self.enter()?;
        let mut left = self.atom(run)?;
        loop {
            self.peek();
            let Some((op, precedence)) = [ ("||",1), ("&&",2), ("==",3), ("!=",3), ("<=",4), (">=",4), ("<",4), (">",4),
                ("+",5), ("-",5), ("*",6), ("/",6), ("%",6) ].iter()
                .find(|(op, p)| *p >= minimum && self.source[self.pos..].starts_with(op)).copied() else { break; };
            self.step(1)?; self.pos += op.len();
            let a = left.int()?;
            let active = run && !(op == "&&" && a == 0 || op == "||" && a != 0);
            let right = self.expr(precedence + 1, active)?;
            let b = if active { right.int()? } else { 0 };
            left = Value::Int(if !run { 0 } else { match op {
                "+" => a.wrapping_add(b), "-" => a.wrapping_sub(b), "*" => a.wrapping_mul(b),
                "/" | "%" if b == 0 => return Err(Error("division by zero")),
                "/" => a.wrapping_div(b), "%" => a.wrapping_rem(b),
                "==" => i64::from(a == b), "!=" => i64::from(a != b), "<" => i64::from(a < b), ">" => i64::from(a > b),
                "<=" => i64::from(a <= b), ">=" => i64::from(a >= b),
                "&&" => i64::from(a != 0 && b != 0), "||" => i64::from(a != 0 || b != 0), _ => 0,
            }});
        }
        self.depth -= 1; Ok(left)
    }
    fn atom(&mut self, run: bool) -> Result<Value, Error> {
        self.step(1)?;
        if self.take("-") { return Ok(Value::Int(self.expr(7, run)?.int()?.wrapping_neg())); }
        if self.take("+") { return self.expr(7, run); }
        if self.take("!") { return Ok(Value::Int(i64::from(self.expr(7, run)?.int()? == 0))); }
        if self.take("(") { let value = self.expr(0, run)?; self.need(")")?; return Ok(value); }
        if self.take("\"") {
            let mut text = Text::EMPTY;
            loop {
                let b = *self.source.as_bytes().get(self.pos).ok_or(Error("unterminated string"))?; self.pos += 1;
                if b == b'"' { break; }
                let b = if b == b'\\' {
                    let escape = *self.source.as_bytes().get(self.pos).ok_or(Error("unterminated escape"))?; self.pos += 1;
                    match escape { b'n' => b'\n', b't' => b'\t', b'"' => b'"', b'\\' => b'\\', _ => return Err(Error("unknown string escape")) }
                } else { b };
                text.push(b)?;
            }
            return Ok(if run { Value::Str(text) } else { Value::Int(0) });
        }
        if self.peek().is_some_and(|b| b.is_ascii_digit()) {
            let radix = if self.take("0x") || self.take("0X") { 16 } else { 10 };
            let start = self.pos; let mut number = 0u64;
            while let Some(b) = self.source.as_bytes().get(self.pos) {
                let digit = match b { b'0'..=b'9' => b - b'0', b'a'..=b'f' => b - b'a' + 10, b'A'..=b'F' => b - b'A' + 10, _ => break };
                if digit as u64 >= radix { break; }
                number = number.checked_mul(radix).and_then(|v| v.checked_add(digit as u64)).ok_or(Error("number too large"))?;
                self.pos += 1;
            }
            if self.pos == start { return Err(Error("expected number")); }
            return Ok(Value::Int(number as i64));
        }
        let mut name = [0u8; 24];
        let id = self.ident().ok_or(Error("expected expression"))?;
        if id.len() > name.len() { return Err(Error("identifier too long")); }
        let len = id.len(); name[..len].copy_from_slice(id.as_bytes());
        let name = core::str::from_utf8(&name[..len]).unwrap_or("");
        let call = self.take("(");
        if !call {
            if let Some(value) = self.state.get(name) { return Ok(if run { value } else { Value::Int(0) }); }
        }
        let mut args = [Value::Void; 5]; let mut count = 0;
        if call {
            if self.peek() != Some(b')') {
                loop {
                    if count == args.len() { return Err(Error("too many arguments")); }
                    args[count] = if matches!(self.peek(), Some(b',' | b')')) { Value::Void } else { self.expr(0, run)? };
                    count += 1; if !self.take(",") { break; }
                }
            }
            self.need(")")?;
        }
        if !run { return Ok(Value::Int(0)); }
        self.call(builtin(name).ok_or(Error("unknown variable or function"))?, &args[..count])
    }
    fn emit(&mut self, text: &str) -> Result<(), Error> {
        self.output = self.output.checked_sub(text.len()).ok_or(Error("output limit reached"))?;
        print(text); Ok(())
    }
    fn emit_value(&mut self, value: Value) -> Result<(), Error> {
        match value {
            Value::Str(s) => self.emit(s.str()),
            Value::Int(n) => { let mut buf = [0; 20]; self.emit(calc::format(n, &mut buf))?; self.emit("\n") },
            Value::Void => Ok(()),
        }
    }
    fn call(&mut self, id: usize, args: &[Value]) -> Result<Value, Error> {
        if id != 4 && id != 5 && !args.is_empty() { return Err(Error("expected no arguments")); }
        match id {
            0..=3 => { self.step(100)?; (self.context.command)(["help", "about", "crow", "clear"][id]); }
            4 => {
                if args.len() != 1 { return Err(Error("Print expects one argument")); }
                self.emit_value(args[0])?;
            }
            5 => {
                if !(4..=5).contains(&args.len()) { return Err(Error("Draw expects x,y,w,h[,color]")); }
                let (x,y,w,h) = (args[0].int()?,args[1].int()?,args[2].int()?,args[3].int()?);
                let color = match args.get(4) { None | Some(Value::Void) => 0xffffff, Some(v) => v.int()? };
                if [x,y,w,h,color].iter().any(|n| *n < 0) || color > 0xffffff { return Err(Error("invalid Draw coordinate or colour")); }
                let (x,y,w,h) = (x.min(800) as usize,y.min(600) as usize,w.min(800) as usize,h.min(600) as usize);
                self.step(1 + w * h / 256)?;
                (self.context.graphics)(Effect::Draw(x,y,w,h,color as u32));
            }
            6 => for i in 0..self.state.vars.len() {
                let v = self.state.vars[i]; if v.len == 0 { continue; }
                self.emit(core::str::from_utf8(&v.name[..v.len]).unwrap_or(""))?; self.emit(" = ")?;
                self.emit_value(v.value)?; if matches!(v.value, Value::Str(_)) { self.emit("\n")?; }
            },
            7 => self.state.vars = [Variable::EMPTY; 64],
            8 => return Ok(Value::Int(self.context.ticks.min(i64::MAX as u64) as i64)),
            _ => return Err(Error("unknown builtin")),
        }
        Ok(Value::Void)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_precedes_execution() {
        let context = Context { graphics: |_| {}, command: |_| true, ticks: 0, cancelled: || true };
        assert_eq!(CrowC::new().eval("while(1){}", context), Err(Error("interrupted")));
    }
}
