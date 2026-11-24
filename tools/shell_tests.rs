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
    s.run_as_typed("hlep"); let out = take();
    assert!(out.contains("Did you mean `help`?")); assert!(!out.contains("Not technical?"));
    s.run_as_typed("wut"); assert!(take().contains("Not technical? Type `dio`"));
    s.run_as_typed("help"); take();
    s.run_as_typed("oops"); assert!(!take().contains("Not technical?"));
    s.run_as_typed("echo remembered"); take();
    input(&mut s, b"\x1b[A\r"); assert!(take().contains("remembered\n"));
    input(&mut s, b"echo draft\x1b[A\x1b[B\r"); assert!(take().contains("draft\n"));
    input(&mut s, b"ec\tcompleted\r"); assert!(take().contains("completed\n"));
    input(&mut s, b"h\t"); let out = take(); assert!(out.contains("help") && out.contains("history"));
    input(&mut s, b"\x03"); take();
    for n in 0..20 { s.run_as_typed(&format!("echo slot{n}")); take(); }
    s.run_as_typed("history"); let out = take();
    assert!(!out.contains("echo slot0\n")); assert!(out.contains("echo slot19\n"));

    s.tick(100_000); s.run_as_typed("uptime"); assert!(take().contains("100000 ms"));
    s.run_as_typed("mem"); assert!(take().contains("Free RAM is not tracked"));
    s.run_as_typed("color 15"); assert!(take().contains("\x1b[97m"));
    s.run_as_typed("color 16"); assert!(take().contains("usage: color"));
    s.run_as_typed("repeat 3 echo ping"); assert_eq!(take().matches("ping\n").count(), 4);
    s.run_as_typed("repeat 32 repeat 32 help"); assert!(take().contains("no nested repeat"));
    s.run_as_typed("sleep 100"); assert!(!take().ends_with("corvus> "));
    s.tick(100_099); assert_eq!(take(), "");
    s.tick(100_100); assert_eq!(take(), "corvus> ");
    s.run_as_typed("sleep 60000"); take(); input(&mut s, b"\x03");
    assert_eq!(take(), "^C\ncorvus> "); s.tick(200_000); assert_eq!(take(), "");
    s.run_as_typed("sleep 60001"); assert!(take().contains("usage: sleep"));
    s.run_as_typed("lantern"); assert!(take().contains("DIOGENES"));
    s.run_as_typed("banner noctis"); assert!(take().contains("noctis\n"));
    s.run_as_typed("logo"); assert!(take().contains("C O R V U S"));

}
