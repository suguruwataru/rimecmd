fn main() {
    println!("cargo:rustc-link-search=build/meson");
    println!("cargo:rustc-link-lib=rimed");
    println!("cargo:rustc-link-lib=rime");
    println!("cargo:rerun-if-changed=build/meson/librimed.a");
}
