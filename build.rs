fn main() {
    let target = std::env::var("TARGET").expect("TARGET not set by cargo");

    cc::Build::new()
        .file("src/cxx/bithash.c")
        .include("src/cxx")
        .opt_level(3)
        .target(&target)
        .compile("bithash");

    println!("cargo:rerun-if-changed=src/cxx/bithash.c");
    println!("cargo:rerun-if-changed=src/cxx/bithash.h");
}
