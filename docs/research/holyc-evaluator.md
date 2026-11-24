# CrowC: a tiny HolyC-inspired evaluator for Corvus OS

Design note, 2026-11-24. Target: bare-metal ARM64, `no_std`, no global allocator, serial shell + framebuffer terminal.
Inspired by HolyC's *idea* (the prompt is the language); no TempleOS code or syntax copied verbatim.

## What we borrow from HolyC (concepts only)
- The shell is the language: top-level statements run immediately, no `main` [1][2].
- A bare string literal statement prints itself: `"Hello\n";` [1].
- A call with no (or only default) args can drop the parens: `Help;` instead of `Help();` [1].
- Default args anywhere in a parameter list, skipped with a leading comma: `Foo(,6)` [1].
- TempleOS JIT-compiles HolyC to x86-64 [2]. We do NOT JIT: a tree-walking/bytecode interpreter is
  enough, safer on ARM64 (no I-cache maintenance, no W^X dance), and keeps the kernel small.

## Goals
- Type `1+2*3` at the prompt, get `7`. Type `"caw";`, get `caw`. Type `Crow;`, get the crow.
- Zero heap: all state in fixed-size static arenas; errors (not panics) on exhaustion.
- One line in, one result out; the existing commands keep working unchanged.
- Small: target < ~1.5k lines of Rust for M1-M3, no external deps except optionally `heapless`.
- Deterministic and interruptible: step budget per line so `while (1);` cannot hang the kernel.

## Non-goals (for now)
- Types beyond `i64` and string; floats, structs, pointers, classes.
- JIT / native code, a real C compiler, file includes, garbage collection.
- Multi-line editing beyond `{ ... }` continuation; persistence across reboots.

## Grammar (EBNF-ish)
```
line     := stmt* EOF
stmt     := "let" IDENT "=" expr ";"?            (M2)
          | IDENT "=" expr ";"?                  (M2)
          | "if" "(" expr ")" block ("else" block)?   (M3)
          | "while" "(" expr ")" block           (M3)
          | "fn" IDENT "(" params? ")" block     (M4)
          | "return" expr? ";"?                  (M4)
          | STRING ";"?                          -> implicit Print
          | expr ";"?                            -> value echoed if non-void
block    := "{" stmt* "}"
params   := IDENT ("=" expr)? ("," IDENT ("=" expr)?)*
expr     := cmp ( ("&&" | "||") cmp )*
cmp      := sum ( ("==" | "!=" | "<" | "<=" | ">" | ">=") sum )?
sum      := term ( ("+" | "-") term )*
term     := unary ( ("*" | "/" | "%") unary )*
unary    := ("-" | "!") unary | call
call     := IDENT "(" args? ")" | IDENT        // bare IDENT: var if bound, else 0-arg call
          | primary
args     := expr? ("," expr?)*                 // empty slot = use default
primary  := INT | HEXINT | STRING | "(" expr ")"
```
Semicolons are optional at end of line (friendlier than HolyC). Integers are `i64`, wrapping ops,
`/0` is a runtime error. Strings are immutable slices into the string arena.

## String literals and output
- A statement that is only a string literal prints it verbatim (escapes `\n \t \" \\` processed). No auto newline;
  the shell adds one if the cursor is not at column 0 after the line, so `"caw"` looks right at the prompt.
- Optional (later): `"x=%d\n", x;` = HolyC-style format list, i.e. sugar for `Print("x=%d\n", x)`.
- Bare expression statements echo their value (`7`), like a calculator; `void` builtins echo nothing.

## Mapping existing commands
Old-style commands become builtins, and a compatibility pre-pass keeps the old syntax alive:

| Typed today      | Becomes                         | Notes                                  |
|------------------|---------------------------------|----------------------------------------|
| `help`           | `Help;`                         | lists builtins + user fns              |
| `about`          | `About;`                        |                                        |
| `crow`           | `Crow;`                         |                                        |
| `echo hi there`  | `Print("hi there")`             | pre-pass: first word `echo` = raw text |
| -                | `Draw(x, y, w, h, color)`       | framebuffer rect; `color` default 0xFFFFFF |
| -                | `Cls;`, `Peek(addr)`, `Ticks;`  | handy kernel builtins (Peek gated)     |

Pre-pass rule: if the line's first word is a lowercase legacy command name and the line does not parse as
an expression, dispatch to the old handler. Lookup of identifiers is case-insensitive for builtins so
`crow` and `Crow` both work. Builtins live in a `static BUILTINS: [Builtin; N]` table:
`struct Builtin { name: &'static str, arity: u8, defaults: &'static [i64], f: fn(&mut Ctx, &[Val]) -> Result<Val, Err> }`.

## Memory model (no alloc)
All arenas are `static` (or one `Interp` struct in `.bss`), sizes as const generics so they are tunable:

| Arena        | Type                                  | Default size | Lifetime            |
|--------------|---------------------------------------|--------------|---------------------|
| Line buffer  | `[u8; 256]`                           | 256 B        | per line            |
| Tokens       | `[Tok; 128]` (kind + span u16)        | ~1 KiB       | per line            |
| AST / code   | `[Node; 512]`, children as `u16` idx  | ~8 KiB       | per line / per fn   |
| Value stack  | `[Val; 64]`                           | 1 KiB        | per line            |
| Variables    | `[(Sym, Val); 64]` linear scan        | 1 KiB        | session             |
| Interned str | `[u8; 4096]` bump + `[Span; 128]`     | 4 KiB        | session (reset cmd) |
| Functions    | `[FnDef; 16]` pointing into code arena| small        | session             |

- `Val` = `enum { Int(i64), Str(StrId), Void }` (16 bytes). No references, no GC.
- Two-region bump allocation: the per-line region is reset after each line; the session region only grows
  and is cleared with `Reset;`. User function bodies are copied (compacted) from the line region into
  the session region at definition time.
- No recursion in the parser beyond a fixed depth (e.g. 32) -> `Err::TooDeep`, protecting the kernel stack.
- Every exhaustion is a typed `Err` (`OutOfNodes`, `OutOfStrings`, `StepBudget`), printed and recovered.
- Ctrl-C on serial sets an atomic flag checked every N steps alongside the step budget.

## Staged plan
- **M1 calculator**: lexer + Pratt/precedence-climbing parser, eval ints, `+ - * / % ()` and comparisons,
  string-literal print, builtins with/without parens (`Help`, `About`, `Crow`, `Print`), legacy pre-pass.
  Unit-test the core on host (`cargo test` with the crate being `no_std` + `#[cfg(test)] extern crate std`).
- **M2 variables**: `let`/assignment, symbol interning, session arena, `Vars;` and `Reset;` builtins,
  `Draw(x,y,w,h,c)` with default args and `Draw(10,10,50,50)` working from the prompt.
- **M3 control flow**: `{}` blocks with multi-line continuation (prompt shows `..` until braces balance),
  `if/else`, `while`, `&& ||`, step budget + Ctrl-C. Demo: `i=0; while(i<5){Draw(i*20,0,15,15); i=i+1}`.
- **M4 user functions**: `fn Name(a, b=2) { ... }`, `return`, call frames in a `[Frame; 16]` stack,
  paren-less call of 0-arg user fns, HolyC-style `Foo(,6)` default skipping, `Fns;` listing.
- Later: compile AST to compact bytecode (same arenas), format-list print sugar, script files from a
  ramdisk, an allocator-backed "big mode" once the kernel gets a heap.

## Reuse options (verified 2026-11-24)
| Option | Version | License | no_std w/o alloc? | Verdict |
|---|---|---|---|---|
| rhai | 1.27.0 | MIT OR Apache-2.0 | No: `no_std` feature still needs a global allocator; sample uses nightly [3] | Revisit after heap exists |
| stak (Scheme) | 0.12.35 | MIT | Claims no-std and no-alloc support [4] | Best "drop-in" candidate, but Scheme syntax, not HolyC feel |
| grift (Lisp) | 7.0.0 (1.x was no_alloc) | MIT OR Apache-2.0 | 1.3.0: yes, fixed `Lisp<N>` arena + mark-sweep [5]; 7.0.0: allocator-backed [6] | Read 1.3.0 source as arena reference |
| exp-rs | 0.2.0 | MIT OR Apache-2.0 | `no_std`, alloc not stated; examples use `Vec`/`String` [7] | Floats/math only; reference for juxtaposition calls (`sin x`) |
| scriptful | 0.4.0 | MIT OR Apache-2.0 | `no_std` stack machine | Reference for an op/value stack design |
| a4-core (Forth) | 0.0.4 (2021) | MIT OR Apache-2.0 | no_std, bytecode | Stale; idea reference only |
| heapless | 0.9.4 | MIT OR Apache-2.0 | Yes, `static` friendly `Vec`/`String`/`IndexMap` [8] | Use for arenas instead of hand-rolled |
| tinyexpr (C) | master | zlib | Uses `malloc` per AST node [9][10] | Reference for precedence parsing; not linkable as-is |
| TempleOS / HolyC | - | Public domain [2] | n/a (x86-64, JIT) | Read for ideas only; we don't copy code (owner's choice) |

**Recommendation**: hand-write the evaluator (the grammar above is small), use `heapless` for the fixed
containers, and keep stak / grift 1.3 as references for arena layout and GC-free design. Adopting rhai
makes sense only once Corvus OS has an allocator, and it would replace the HolyC feel with Rust-like syntax.

## Sources
1. https://harrison.totty.dev/p/a-lang-design-analysis-of-holyc (HolyC language design analysis)
2. https://en.wikipedia.org/wiki/TempleOS
3. https://rhai.rs/book/start/builds/no-std.html ; https://crates.io/crates/rhai
4. https://docs.rs/crate/stak/latest ; https://github.com/raviqqe/stak
5. https://docs.rs/crate/grift/1.3.0
6. https://docs.rs/crate/grift/latest
7. https://lib.rs/crates/exp-rs ; https://crates.io/crates/exp-rs
8. https://crates.io/crates/heapless ; https://crates.io/crates/scriptful ; https://crates.io/crates/a4-core
9. https://codeplea.com/tinyexpr
10. https://github.com/codeplea/tinyexpr (tinyexpr.c, LICENSE)
