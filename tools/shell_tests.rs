#[path = "../src/calc.rs"] mod calc;
#[path = "../src/shell.rs"] mod shell;
use std::sync::Mutex;
static DRAW: Mutex<Vec<(usize,usize,usize,usize,u32)>> = Mutex::new(Vec::new());
fn draw(effect: shell::platform::Effect) { if let shell::platform::Effect::Draw(x,y,w,h,c) = effect { DRAW.lock().unwrap().push((x,y,w,h,c)); } }
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

    s.graphics = draw;
    s.run_as_typed("\"caw\";"); assert!(take().contains("caw\n"));
    s.run_as_typed("Help;"); assert!(take().contains("help  about"));
    s.run_as_typed("1+2*3"); assert!(take().contains("7\n"));
    s.run_as_typed("let x = 9; x * 2"); assert!(take().contains("18\n"));
    s.run_as_typed("let message = \"plain\"; message;"); assert!(take().contains("plain\n"));
    s.run_as_typed("if (x == 9) { \"yes\"; } else { 1/0; }"); assert!(!take().contains("division by zero"));
    s.run_as_typed("0 && (1/0); 1 || (1/0)"); assert!(!take().contains("division by zero"));
    s.run_as_typed("i=0; while(i<5){Draw(i*20,0,15,15,14); i=i+1}");
    assert!(!take().contains("CrowC:")); assert_eq!(DRAW.lock().unwrap().len(),5);
    assert_eq!(DRAW.lock().unwrap()[4],(80,0,15,15,14));
    s.run_as_typed("Draw(790,590,1000,1000)"); assert!(!take().contains("CrowC:"));
    s.run_as_typed("while(1){}"); assert!(take().contains("step limit reached"));
    s.run_as_typed("1/0"); assert!(take().contains("division by zero"));
    s.run_as_typed("while(0){"); assert!(take().ends_with(".. "));
    s.run_as_typed("1/0;"); assert!(take().ends_with(".. "));
    s.run_as_typed("}"); assert!(!take().contains("division by zero"));
    s.run_as_typed("if(1){"); take(); input(&mut s,b"\x03"); assert!(take().ends_with("corvus> "));
    s.run_as_typed("Reset;"); take(); s.run_as_typed("x"); assert!(take().contains("isn't a command"));
    for n in 0..64 { s.run_as_typed(&format!("v{n}=1")); assert!(!take().contains("CrowC:")); }
    s.run_as_typed("overflow=1"); assert!(take().contains("64 variables"));
    s.run_as_typed("Reset;"); take();
    s.run_as_typed(&format!("{}1{}", "(".repeat(40), ")".repeat(40))); assert!(take().contains("nesting too deep"));

    let alphabet = b"abcletifwh0129 +-*/%(){};=<>!&|\"";
    let mut seed = 7u32;
    for _ in 0..512 {
        let mut probe = shell::Shell::new();
        let mut source = String::new();
        for _ in 0..80 { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); source.push(alphabet[seed as usize % alphabet.len()] as char); }
        probe.run_as_typed(&source); take();
    }

    s.run_as_typed("dio"); take(); s.action("help"); let out = take();
    assert!(out.contains("help  about") && out.ends_with("corvus> "));

    input(&mut s,b"\x1eDIO_STATUS ready\x1f"); assert!(s.dio_online); assert_eq!(take(), "");
    s.run_as_typed("dio why"); take(); input(&mut s,b"\x1eDIO_STATUS offline\x1f");
    assert!(!s.dio_online); assert_eq!(take(), "");
    input(&mut s,b"\x1ethe answer\x1f"); assert!(take().contains("the answer"));

}
