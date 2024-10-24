fn main() {
    println!("cargo:rustc-link-search=build/meson");
    println!("cargo:rustc-link-lib=rimecmd");
    println!("cargo:rustc-link-lib=rime");
    println!("cargo:rerun-if-changed=build/meson/librimecmd.a");
}
