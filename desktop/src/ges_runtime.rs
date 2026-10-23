//! Configure plugin discovery before Tauri starts any threads.
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

fn packaged_root(executable: &Path) -> Result<PathBuf> {
    let directory = executable.parent().context("无法定位媒体运行库目录")?;
    #[cfg(windows)]
    return Ok(directory.join("gstreamer"));
    #[cfg(not(windows))]
    Ok(directory
        .parent()
        .context("无法定位应用资源目录")?
        .join("Resources/gstreamer"))
}

fn plugins(packaged: &Path, development: Option<&Path>) -> Result<PathBuf> {
    let bundled = packaged.join("lib/gstreamer-1.0");
    if packaged.join("runtime.json").is_file() && bundled.is_dir() {
        return Ok(bundled);
    }
    if let Some(development) = development {
        ensure!(development.is_dir(), "开发环境缺少 GStreamer 插件目录");
        return Ok(development.to_owned());
    }
    anyhow::bail!("应用包缺少 GStreamer 媒体运行库，请重新安装完整应用")
}

#[cfg(debug_assertions)]
fn development_plugins() -> PathBuf {
    #[cfg(windows)]
    if let Some(sdk) = std::env::var_os("GSTREAMER_1_0_ROOT_MSVC_X86_64") {
        return PathBuf::from(sdk).join("lib/gstreamer-1.0");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native/ges-dev-plugins")
}

pub fn configure() -> Result<()> {
    // The bundled compositor's ORC-generated alpha blend crashes in the
    // hardened macOS arm64 app (blend_pads -> JIT code), despite passing in
    // the test binary. Use ORC's C fallback before any GStreamer threads start.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        std::env::set_var("ORC_CODE", "backup");
    }
    let packaged = packaged_root(&std::env::current_exe()?)?;
    #[cfg(debug_assertions)]
    let development = Some(development_plugins());
    #[cfg(not(debug_assertions))]
    let development: Option<PathBuf> = None;
    let plugins = plugins(&packaged, development.as_deref())?;
    // SAFETY: called only as the first operation of main, before threads.
    unsafe {
        std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", plugins);
        std::env::set_var("GST_PLUGIN_PATH_1_0", "");
        // Avoid relying on an external scanner executable outside the bundle.
        std::env::set_var("GST_REGISTRY_FORK", "no");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_requires_bundled_plugins_even_on_a_development_machine() {
        let root = std::env::temp_dir().join(format!("ges-runtime-{}", mstudio::media::id()));
        let packaged = root.join("bundle");
        let development = root.join("development");
        std::fs::create_dir_all(&development).unwrap();
        assert!(plugins(&packaged, None).is_err());
        assert_eq!(plugins(&packaged, Some(&development)).unwrap(), development);
        let bundled = packaged.join("lib/gstreamer-1.0");
        std::fs::create_dir_all(&bundled).unwrap();
        assert!(plugins(&packaged, None).is_err());
        std::fs::write(packaged.join("runtime.json"), "{}").unwrap();
        assert_eq!(plugins(&packaged, None).unwrap(), bundled);
        assert_eq!(plugins(&packaged, Some(&development)).unwrap(), bundled);
        std::fs::remove_dir_all(root).unwrap();
    }
}
