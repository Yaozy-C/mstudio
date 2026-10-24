fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let library =
            std::env::var("DEP_MLX_METALLIB").expect("MLX must publish its compiled Metal library");
        println!("cargo:metallib={library}");
        println!("cargo:rustc-env=MSTUDIO_MLX_METALLIB={library}");
        cc::Build::new()
            .cpp(true)
            .file("native/shader.cpp")
            .compile("mstudio_mlx_shader");
        println!("cargo:rerun-if-changed=native/shader.cpp");
    }
}
