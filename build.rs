// build.rs
use std::env;
use std::path::Path;

fn main() {
    // Path to C source files
    let c_src_path = Path::new("src/c");
    
    // Rebuild if source files change
    println!("cargo:rerun-if-changed=src/c/udp_socket.c");
    println!("cargo:rerun-if-changed=src/c/udp_socket.h");
    
    // Compile C code
    cc::Build::new()
        .file(c_src_path.join("udp_socket.c"))
        .include(c_src_path)
        .flag("-fPIC")
        .compile("udp_socket");
    
    // Link VMA library
    println!("cargo:rustc-link-lib=vma");
    
    // Set output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir);
}