fn main() {
    println!("cargo:rerun-if-env-changed=MLT_SDK");
    println!("cargo:rerun-if-changed=native/player.cpp");
    println!("cargo:rerun-if-changed=native/player.h");
    println!("cargo:rerun-if-changed=native/surface.h");
    println!("cargo:rerun-if-changed=native/surface_mac.mm");
    println!("cargo:rerun-if-changed=native/surface_windows.cpp");
    let sdk = std::env::var_os("MLT_SDK")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("native/runtime")
        });
    assert!(
        sdk.join("include/mlt-7/mlt++/Mlt.h").exists(),
        "MLT SDK missing: run scripts/build-mlt.py or set MLT_SDK"
    );
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .include(sdk.join("include/mlt-7"))
        .file("native/player.cpp");
    match os.as_str() {
        "macos" => {
            build.file("native/surface_mac.mm").flag("-fobjc-arc");
            for name in ["AppKit", "QuartzCore", "CoreGraphics"] {
                println!("cargo:rustc-link-lib=framework={name}");
            }
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Resources/mlt/lib");
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{}",
                sdk.join("lib").display()
            );
        }
        "windows" => {
            build
                .file("native/surface_windows.cpp")
                .define("NOMINMAX", None);
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=user32");
        }
        _ => panic!("Native preview currently supports macOS and Windows"),
    }
    build.compile("mstudio_mlt");
    println!(
        "cargo:rustc-link-search=native={}",
        sdk.join("lib").display()
    );
    println!("cargo:rustc-link-lib=dylib=mlt++-7");
    println!("cargo:rustc-link-lib=dylib=mlt-7");
    tauri_build::build()
}
