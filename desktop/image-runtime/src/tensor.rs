use anyhow::{Result, ensure};
use mlx_rs::{Array, Dtype, Stream};

pub fn run_inference<T>(run: impl FnOnce() -> Result<T>) -> Result<T> {
    let result = run();
    // Completion handlers can retain model buffers after their tensors drop.
    let synchronized = synchronize();
    mlx_rs::memory::clear_cache();
    tracing::info!(target: "mstudio_image_runtime", phase = "cleanup",
        active_bytes = mlx_rs::memory::get_active_memory(), cache_bytes = mlx_rs::memory::get_cache_memory(),
        success = result.is_ok(), "MLX inference memory");
    result.and_then(|output| {
        synchronized?;
        Ok(output)
    })
}

fn synchronize() -> Result<()> {
    let mut success = true;
    for stream in [Stream::cpu(), Stream::gpu()] {
        success &= unsafe { mlx_sys::mlx_synchronize(stream.as_ptr()) } == 0;
    }
    ensure!(success, "MLX could not finish releasing inference buffers");
    Ok(())
}

/// Absolute MLX allocations, not process RSS. Each phase includes model loading.
pub fn phase<T>(name: &str, run: impl FnOnce() -> Result<T>) -> Result<T> {
    synchronize()?;
    mlx_rs::memory::clear_cache();
    let baseline = mlx_rs::memory::get_active_memory();
    mlx_rs::memory::reset_peak_memory();
    let result = run();
    let synchronized = synchronize();
    tracing::info!(target: "mstudio_image_runtime", phase = name, baseline_bytes = baseline,
        peak_bytes = mlx_rs::memory::get_peak_memory(), active_bytes = mlx_rs::memory::get_active_memory(),
        success = result.is_ok(), "MLX inference memory");
    mlx_rs::memory::clear_cache();
    result.and_then(|output| {
        synchronized?;
        Ok(output)
    })
}

/// MLX's raw host slice ignores strides. Materialize row-major float32 before reading it.
pub fn host_array(array: &Array) -> Result<Array> {
    let array = array.as_dtype(Dtype::Float32)?;
    // MLX allocates the handle and transfers its ownership to Array, including on errors.
    let mut raw = unsafe { mlx_sys::mlx_array_new() };
    let status =
        unsafe { mlx_sys::mlx_contiguous(&mut raw, array.as_ptr(), false, Stream::new().as_ptr()) };
    let result = unsafe { Array::from_ptr(raw) };
    ensure!(status == 0, "MLX could not materialize a row-major tensor");
    result.eval()?;
    Ok(result)
}
