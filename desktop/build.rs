fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let library = std::env::var("DEP_MSTUDIO_IMAGE_METALLIB")
            .expect("Native image runtime must publish its Metal library");
        std::fs::create_dir_all("native/mlx").expect("create MLX resource directory");
        std::fs::copy(library, "native/mlx/mlx.metallib")
            .expect("stage the exact compiled MLX shaders");
        cc::Build::new()
            .file("native/metal_preview.m")
            .flag("-fobjc-arc")
            .flag("-fmodules")
            .flag(format!(
                "-fmodules-cache-path={}/clang-modules",
                std::env::var("OUT_DIR").unwrap()
            ))
            .compile("mstudio_metal_preview");
        for framework in ["AppKit", "Metal", "QuartzCore", "CoreGraphics"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rerun-if-changed=native/metal_preview.m");
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
