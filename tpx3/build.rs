fn main() {
    // Where to find TTX.so at link time
    println!("cargo:rustc-link-search=native=lib");

    // Linux runtime loader path
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/lib");

    // Tell Rust which library to link
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=dylib=TTX");      // TTX.dll
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=dylib=TTX");      // Rust will look in lib/ for libTTX.so
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=dylib=TimeTagger");      // Rust will look in lib/ for libTimeTagger.so
}

