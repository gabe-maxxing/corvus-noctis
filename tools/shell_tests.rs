#[path = "../src/calc.rs"] mod calc;
#[path = "../src/shell.rs"] mod shell;
use std::sync::Mutex;
static OUTPUT: Mutex<String> = Mutex::new(String::new());
fn print(text: &str) { OUTPUT.lock().unwrap().push_str(text); }
fn take() -> String { std::mem::take(&mut *OUTPUT.lock().unwrap()) }
fn input(shell: &mut shell::Shell, bytes: &[u8]) { for b in bytes { shell.feed(*b); } }

#[test]
fn shell_contract() {
    shell::banner(); assert!(!take().is_empty());
    let mut s = shell::Shell::new();
    s.run_as_typed("dio"); assert!(take().ends_with("diogenes> "));
    input(&mut s, b"what is virtue?\r\n");
    assert!(take().ends_with("\x1eDIO what is virtue?\n"));
    input(&mut s, b"\x1eLive plainly.\x1f");
    assert_eq!(take(), "Live plainly.\ndiogenes> "); assert!(s.dio_online);
    s.run_as_typed("exit"); assert!(take().ends_with("corvus> "));
    s.run_as_typed("? why"); assert!(take().ends_with("\x1eDIO why\n"));
    input(&mut s, b"\x03"); assert_eq!(take(), "^C\ncorvus> ");
    input(&mut s, b"\x1eecho late reply\x1f"); assert_eq!(take(), "");
    s.run_as_typed("dio time"); take(); s.tick(45_000);
    assert!(take().contains("timed out")); assert!(!s.dio_online);
    input(&mut s, b"\x1elate timeout\x1f"); assert_eq!(take(), "");
    s.run_as_typed("dio hello"); take();
    input(&mut s, b"\x1eDiogenes offline: install Ollama\x1f");
    assert!(take().contains("install Ollama")); assert!(!s.dio_online);
    s.run_as_typed("calc 6*7"); assert!(take().contains("42\n"));
    s.run_as_typed("help"); assert!(take().contains("help  about"));
}
