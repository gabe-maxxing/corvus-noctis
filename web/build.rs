//! Embeds ../assets/corvus.cvf when present (personal artwork, gitignored),
//! exactly like the kernel's build.rs. Without it the console has no logo.
use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=../assets/corvus.cvf");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("corvus.cvf");
    match fs::read("../assets/corvus.cvf") {
        Ok(bytes) => fs::write(out, bytes).unwrap(),
        Err(_) => fs::write(out, []).unwrap(),
    }
}
