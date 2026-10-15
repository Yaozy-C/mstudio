//! Owns an embedded CAMetalLayer; AppKit changes are dispatched through Tauri.
use crate::preview_surface::Bounds;
use std::{
    ffi::{CStr, c_char, c_void},
    sync::{Arc, Mutex},
};
unsafe extern "C" {
    fn mstudio_metal_create(
        view: *mut c_void,
        width: u32,
        height: u32,
        error: *mut c_char,
        capacity: usize,
    ) -> *mut c_void;
    fn mstudio_metal_bounds(
        handle: *mut c_void,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        viewport_w: f64,
        viewport_h: f64,
        show: bool,
    );
    fn mstudio_metal_present(handle: *mut c_void, pixels: *const u8, generation: u64) -> i32;
    fn mstudio_metal_invalidate(handle: *mut c_void, generation: u64);
    fn mstudio_metal_detach(handle: *mut c_void);
    fn mstudio_metal_destroy(handle: *mut c_void);
}
pub struct Surface {
    handle: usize,
    window: tauri::WebviewWindow,
    // Serial CPU uploads. GPU texture slots are independently pinned until completion.
    render: Mutex<()>,
}
impl Surface {
    pub async fn create(
        window: tauri::WebviewWindow,
        width: u32,
        height: u32,
    ) -> Result<Arc<Self>, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .with_webview(move |webview| {
                let mut error = [0 as c_char; 1024];
                let ptr = unsafe {
                    mstudio_metal_create(
                        webview.inner(),
                        width,
                        height,
                        error.as_mut_ptr(),
                        error.len(),
                    )
                };
                let result = if ptr.is_null() {
                    Err(format!(
                        "无法创建 Metal 预览：{}",
                        unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy()
                    ))
                } else {
                    Ok(ptr as usize)
                };
                if let Err(Ok(handle)) = tx.send(result) {
                    unsafe {
                        mstudio_metal_destroy(handle as *mut c_void);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            handle: rx.await.map_err(|e| e.to_string())??,
            window,
            render: Mutex::new(()),
        }))
    }
    pub async fn bounds(self: &Arc<Self>, b: Bounds) -> Result<(), String> {
        if [
            b.x,
            b.y,
            b.width,
            b.height,
            b.viewport_width,
            b.viewport_height,
        ]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 100_000.)
        {
            return Err("预览区域尺寸无效".into());
        }
        let surface = self.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.window
            .run_on_main_thread(move || {
                unsafe {
                    mstudio_metal_bounds(
                        surface.handle as *mut c_void,
                        b.x,
                        b.y,
                        b.width,
                        b.height,
                        b.viewport_width,
                        b.viewport_height,
                        b.visible,
                    );
                }
                let _ = tx.send(());
            })
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|e| e.to_string())
    }
    pub fn invalidate(&self, generation: u64) {
        unsafe {
            mstudio_metal_invalidate(self.handle as *mut c_void, generation);
        }
    }
    pub fn present(&self, pixels: &[u8], generation: u64) -> Result<bool, String> {
        let _render = self.render.lock().unwrap();
        match unsafe {
            mstudio_metal_present(self.handle as *mut c_void, pixels.as_ptr(), generation)
        } {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err("Metal 预览渲染失败，请重新加载预览".into()),
        }
    }
    pub fn detach(self: &Arc<Self>) {
        let surface = self.clone();
        let _ = self.window.run_on_main_thread(move || unsafe {
            mstudio_metal_detach(surface.handle as *mut c_void);
        });
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        let handle = self.handle;
        let _ = self.window.run_on_main_thread(move || unsafe {
            mstudio_metal_destroy(handle as *mut c_void);
        });
    }
}
