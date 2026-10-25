use anyhow::{Result, ensure};
use mlx_rs::{Array, Dtype, Stream};

pub fn run_inference<T>(run: impl FnOnce() -> Result<T>) -> Result<T> {
    let result = run();
    // Completion handlers can retain model buffers after their tensors drop.
    let mut synchronized = true;
    for stream in [Stream::cpu(), Stream::gpu()] {
        synchronized &= unsafe { mlx_sys::mlx_synchronize(stream.as_ptr()) } == 0;
    }
    mlx_rs::memory::clear_cache();
    result.and_then(|output| {
        ensure!(
            synchronized,
            "MLX could not finish releasing inference buffers"
        );
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
