use std::path::Path;
use std::process::Command;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=loader_common");
    Ok(())
}
