use std::path::Path;

fn main() {
    // Path to C source files
    let c_src_path = Path::new("src/c");
    
    // Rebuild if source files change
    println!("cargo:rerun-if-changed=src/c/udp_socket.c");
    println!("cargo:rerun-if-changed=src/c/udp_socket.h");
    println!("cargo:rerun-if-changed=src/c/tcp_socket.c");
    println!("cargo:rerun-if-changed=src/c/tcp_socket.h");
    println!("cargo:rerun-if-changed=src/c/vma_common.c");
    println!("cargo:rerun-if-changed=src/c/vma_common.h");
    
    // Basic build configuration
    let mut common_build = cc::Build::new();
    common_build
        .include(c_src_path)
        .flag("-fPIC")
        .flag("-D_GNU_SOURCE");
    
    // Compile VMA common code
    common_build
        .clone()
        .file(c_src_path.join("vma_common.c"))
        .compile("vma_common");
    
    // Compile UDP socket code
    common_build
        .clone()
        .file(c_src_path.join("udp_socket.c"))
        .compile("udp_socket");
    
    // Compile TCP socket code
    common_build
        .clone()
        .file(c_src_path.join("tcp_socket.c"))
        .compile("tcp_socket");
    
    // Link VMA library - needed for symbols
    println!("cargo:rustc-link-lib=vma");
}