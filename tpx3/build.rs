use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {

    // Preventing re-compiling every time.
    //println!("cargo:rerun-if-changed=build.rs");
    //println!("cargo:rerun-if-changed=lib/TTX.dll");
    //println!("cargo:rerun-if-changed=lib/libTTX.so");

    // Tell Rust where to find native libraries
    println!("cargo:rustc-link-search=native=lib");

    // Windows: link TTX.lib
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=dylib=TTX");

    // Linux: link libTTX.so (assuming your .so is named libTTX.so)
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=dylib=TTX");

    // Resolve target/{debug|release}/ directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // OUT_DIR = target/debug/build/<crate>/out
    // go 3 dirs up → target/debug/ or target/release/
    let target_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("Cannot determine target directory")
        .to_path_buf();

    #[cfg(target_os = "windows")]
    copy_runtime_lib("TTX.dll", &target_dir);

    #[cfg(target_os = "linux")]
    copy_runtime_lib("libTTX.so", &target_dir);
}

/// Copy library from ./lib/ into target dir.
/// Panics with a readable error if missing.
fn copy_runtime_lib(filename: &str, target_dir: &PathBuf) {
    let src = PathBuf::from("lib").join(filename);
    let dest = target_dir.join(filename);

    println!("cargo:warning=Copying {} → {}", src.display(), dest.display());

    fs::copy(&src, &dest)
        .unwrap_or_else(|_| panic!("Failed to copy {} to {}", src.display(), dest.display()));
}
