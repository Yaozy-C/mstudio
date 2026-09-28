//! Configure plugin discovery before Tauri starts any threads.
pub fn configure() {
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
