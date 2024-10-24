fn main() {
    println!("cargo:rustc-link-search=meson_build");
    println!("cargo:rustc-link-lib=rimed");
    println!("cargo:rustc-link-lib=rime");
    println!("cargo:rerun-if-changed=meson_build/librimed.a");
}
