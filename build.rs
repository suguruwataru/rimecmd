fn main() {
    cc::Build::new()
        .file("src/rime_api/rime_api.c")
        .compile("rimecmd");
    println!("cargo:rerun-if-changed=src/rime_api/rime_api.c");
    println!("cargo:rustc-link-lib=rimecmd");
    println!("cargo:rustc-link-lib=rime");
}
