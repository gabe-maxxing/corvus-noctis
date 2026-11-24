//! Embeds the optional bounded logo animation (assets/corvus.cvf, gitignored
//! personal artwork made by tools/gif2frames.py). Without it the kernel still
//! boots and draws the CORVUS title, just without the logo.
use std::{env, fs, path::Path};

const MAX_BYTES: u64 = 512 * 1024;

fn main() {
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=assets/corvus.cvf");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("corvus.cvf");
    let source = Path::new("assets/corvus.cvf");
    match fs::metadata(source) {
        Ok(meta) if meta.len() > MAX_BYTES => {
            panic!("assets/corvus.cvf is {} bytes; the budget is {MAX_BYTES}", meta.len())
        }
        Ok(_) => {
            fs::copy(source, &out).unwrap();
        }
        Err(_) => fs::write(&out, []).unwrap(),
    }
}
