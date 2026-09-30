//! Configure plugin discovery before Tauri starts any threads.
pub fn configure() {
    // The bundled compositor's ORC-generated alpha blend crashes in the
    // hardened macOS arm64 app (blend_pads -> JIT code), despite passing in
    // the test binary. Use ORC's C fallback before any GStreamer threads start.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        std::env::set_var("ORC_CODE", "backup");
    }
    let packaged = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent()?.parent().map(|p| p.join("Resources/gstreamer")));
    let plugins = packaged
        .filter(|p| p.join("lib/gstreamer-1.0").is_dir())
        .map(|p| p.join("lib/gstreamer-1.0"))
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native/ges-dev-plugins")
        });
    if plugins.is_dir() {
        // SAFETY: called only as the first operation of main, before threads.
        unsafe {
            std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", plugins);
            std::env::set_var("GST_PLUGIN_PATH_1_0", "");
            // Avoid relying on an external scanner executable outside the bundle.
            std::env::set_var("GST_REGISTRY_FORK", "no");
        }
    }
}
