use anyhow::{Result, ensure};
use mlx_rs::{Array, Dtype, Stream};

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
