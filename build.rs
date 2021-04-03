use std::path::Path;
use std::process::Command;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let srcs = ["io.s"];
    let lib_name = "myffi";
    for i in 0..srcs.len() {
        let src = Path::new(srcs[i]).file_stem().unwrap().to_str().unwrap();
        Command::new("x86_64-w64-mingw32-gcc")
            .args(&[&format!("src/x86_64/{}.s", src), "-c", "-fPIC", "-o"])
            .arg(&format!("{}/{}.o", out_dir, src))
            .status()
            .unwrap();
        Command::new("x86_64-w64-mingw32-ar")
            .args(&["crus", &format!("{}.lib", lib_name), &format!("{}.o", src)])
            .current_dir(&Path::new(&out_dir))
            .status()
            .unwrap();
    }
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static={}", lib_name);
    Ok(())
}
