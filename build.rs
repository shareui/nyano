fn main() {
    cc::Build::new()
        .file("src/cxx/bithash.c")
        .include("src/cxx")
        .opt_level(3)
        .compile("bithash");
    println!("cargo:rerun-if-changed=src/cxx/bithash.c");
    println!("cargo:rerun-if-changed=src/cxx/bithash.h");
}
