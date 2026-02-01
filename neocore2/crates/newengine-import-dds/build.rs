use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let name = env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set");
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION not set");

    // Cargo package names may contain '-' but Windows dll names are typically with '_'
    // Rust output also uses '_' for the crate file stem.
    let stem = name.replace('-', "_");

    let dll_name = format!("{stem}-{version}.dll");

    // Make sure it's visible in logs
    println!("cargo:warning=Setting DLL output name to {dll_name}");

    // MSVC / lld-link
    println!("cargo:rustc-cdylib-link-arg=/OUT:{dll_name}");
}