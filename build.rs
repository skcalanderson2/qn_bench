fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/QN-Bench.ico");
    #[cfg(target_os = "windows")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        if std::path::Path::new("build/QN-Bench.ico").exists() {
            res.set_icon("build/QN-Bench.ico");
        }
        res.set("ProductName", "QN Bench");
        res.set("FileDescription", "QN Bench — Quantum Networking Optical Bench");
        res.set("LegalCopyright", "Copyright © Al Anderson");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winresource failed: {e}");
        }
    }
}
