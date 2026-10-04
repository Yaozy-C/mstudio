fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Resources/gstreamer/lib");
    }
    println!("cargo:rerun-if-changed=../frontend/src");
    let status = std::process::Command::new("bun")
        .args([
            "run",
            "../frontend/src/domain/build.ts",
            &std::env::var("OUT_DIR").unwrap(),
        ])
        .status()
        .expect("Bun is required to compile the shared domain kernel");
    assert!(status.success(), "domain kernel build failed");
    tauri_build::build()
}
