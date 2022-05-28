use std::{env, path::PathBuf};

fn main() {
    let target = env::var("TARGET").expect("TARGET was not set");

    let dir: PathBuf = ["tree-sitter-yaml", "src"].iter().collect();

    let mut c_config = cc::Build::new();
    c_config.include(&dir);
    c_config.static_flag(true);
    c_config.shared_flag(true);
    c_config
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs");
    let parser_path = dir.join("parser.c");
    c_config.file(&parser_path);
    c_config.compile("parser");
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());

    let mut cpp_config = cc::Build::new();
    cpp_config.cpp(true);
    cpp_config.include(&dir);
    cpp_config.static_flag(true);
    cpp_config
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable");
    let scanner_path = dir.join("scanner.cc");
    cpp_config.file(&scanner_path);
    cpp_config.compile("scanner");
    println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());
    if target.contains("darwin") {
        println!("cargo:rustc-link-lib=static=c++")
    } else {
        println!("cargo:rustc-link-lib=static=stdc++")
    }
}
