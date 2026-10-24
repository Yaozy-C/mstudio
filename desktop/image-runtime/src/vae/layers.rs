//! Static-frame SeedVR2 VAE leaves. NHWC throughout, temporal kernels collapsed at load.
use anyhow::Result;
use mlx_gen::weights::Weights;
use mlx_rs::{Array, Dtype, fast, ops};

pub struct Conv {
    weight: Array,
    bias: Array,
    stride: i32,
    padding: i32,
}
impl Conv {
    pub fn load(w: &Weights, key: &str, stride: i32, padding: i32) -> Result<Self> {
        let raw = w.require(&format!("{key}.weight"))?;
        let weight = raw.sum_axis(2, false)?.transpose_axes(&[0, 2, 3, 1])?;
        let reverse = |n: i32| Array::from_slice(&(0..n).rev().collect::<Vec<_>>(), &[n]);
        let weight = weight
            .take_axis(reverse(weight.shape()[1]), 1)?
            .take_axis(reverse(weight.shape()[2]), 2)?;
        weight.eval()?;
        Ok(Self {
            weight,
            bias: w.require(&format!("{key}.bias"))?.clone(),
            stride,
            padding,
        })
    }
    pub fn forward(&self, x: &Array) -> Result<Array> {
        // Reversed kernels + convolution equal the original cross-correlation. This selects
        // MLX's GEMM convolution, avoiding overflow in its float16 Winograd transforms.
        let y = ops::conv_general(
            x,
            &self.weight,
            &[self.stride][..],
            &[self.padding][..],
            &[1][..],
            &[1][..],
            1,
            true,
        )?;
        Ok(ops::add(y, &self.bias)?)
    }
}

pub struct Norm {
    weight: Array,
    bias: Array,
}
impl Norm {
    pub fn load(w: &Weights, key: &str) -> Result<Self> {
        Ok(Self {
            weight: w
                .require(&format!("{key}.weight"))?
                .as_dtype(Dtype::Float32)?,
            bias: w
                .require(&format!("{key}.bias"))?
                .as_dtype(Dtype::Float32)?,
        })
    }
    pub fn forward(&self, x: &Array) -> Result<Array> {
        let shape = x.shape();
        let (b, c) = (shape[0], shape[shape.len() - 1]);
        let grouped = x
            .as_dtype(Dtype::Float32)?
            .reshape(&[b, -1, 32, c / 32])?
            .transpose_axes(&[0, 2, 1, 3])?
            .reshape(&[b, 32, -1])?;
        let centered = ops::subtract(&grouped, grouped.mean_axis(-1, true)?)?;
        let variance = ops::square(&centered)?.mean_axis(-1, true)?;
        let normalized = ops::multiply(
            &centered,
            ops::add(variance, Array::from_f32(1e-6))?.rsqrt()?,
        )?
        .reshape(&[b, 32, -1, c / 32])?
        .transpose_axes(&[0, 2, 1, 3])?
        .reshape(shape)?;
        Ok(ops::add(ops::multiply(normalized, &self.weight)?, &self.bias)?.as_dtype(x.dtype())?)
    }
}

pub fn silu(x: &Array) -> Result<Array> {
    Ok(ops::multiply(x, ops::sigmoid(x)?)?)
}

pub struct Residual {
    n1: Norm,
    c1: Conv,
    n2: Norm,
    c2: Conv,
    shortcut: Option<Conv>,
}
impl Residual {
    pub fn load(w: &Weights, key: &str) -> Result<Self> {
        Ok(Self {
            n1: Norm::load(w, &format!("{key}.norm1"))?,
            c1: Conv::load(w, &format!("{key}.conv1"), 1, 1)?,
            n2: Norm::load(w, &format!("{key}.norm2"))?,
            c2: Conv::load(w, &format!("{key}.conv2"), 1, 1)?,
            shortcut: if w.get(&format!("{key}.conv_shortcut.weight")).is_some() {
                Some(Conv::load(w, &format!("{key}.conv_shortcut"), 1, 0)?)
            } else {
                None
            },
        })
    }
    pub fn forward(&self, x: &Array) -> Result<Array> {
        let residual = match &self.shortcut {
            Some(c) => c.forward(x)?,
            None => x.clone(),
        };
        let y = self.c1.forward(&silu(&self.n1.forward(x)?)?)?;
        Ok(ops::add(
            self.c2.forward(&silu(&self.n2.forward(&y)?)?)?,
            residual,
        )?)
    }
}

struct Linear {
    weight: Array,
    bias: Array,
}
impl Linear {
    fn load(w: &Weights, key: &str) -> Result<Self> {
        Ok(Self {
            weight: w.require(&format!("{key}.weight"))?.clone(),
            bias: w.require(&format!("{key}.bias"))?.clone(),
        })
    }
    fn forward(&self, x: &Array) -> Result<Array> {
        Ok(ops::add(x.matmul(self.weight.t())?, &self.bias)?)
    }
}

pub struct Mid {
    first: Residual,
    norm: Norm,
    q: Linear,
    k: Linear,
    v: Linear,
    out: Linear,
    last: Residual,
}
impl Mid {
    pub fn load(w: &Weights, key: &str) -> Result<Self> {
        let attn = format!("{key}.attentions.0");
        Ok(Self {
            first: Residual::load(w, &format!("{key}.resnets.0"))?,
            norm: Norm::load(w, &format!("{attn}.group_norm"))?,
            q: Linear::load(w, &format!("{attn}.to_q"))?,
            k: Linear::load(w, &format!("{attn}.to_k"))?,
            v: Linear::load(w, &format!("{attn}.to_v"))?,
            out: Linear::load(w, &format!("{attn}.to_out.0"))?,
            last: Residual::load(w, &format!("{key}.resnets.1"))?,
        })
    }
    pub fn forward(&self, x: &Array) -> Result<Array> {
        let y = self.first.forward(x)?;
        let shape = y.shape();
        let (b, h, w, c) = (shape[0], shape[1], shape[2], shape[3]);
        let n = self.norm.forward(&y)?.reshape(&[b, h * w, c])?;
        // This single-head attention has 512 channels. Accumulate scores in float32.
        let q = self
            .q
            .forward(&n)?
            .reshape(&[b, 1, h * w, c])?
            .as_dtype(Dtype::Float32)?;
        let k = self
            .k
            .forward(&n)?
            .reshape(&[b, 1, h * w, c])?
            .as_dtype(Dtype::Float32)?;
        let v = self
            .v
            .forward(&n)?
            .reshape(&[b, 1, h * w, c])?
            .as_dtype(Dtype::Float32)?;
        let a = fast::scaled_dot_product_attention(&q, &k, &v, (c as f32).powf(-0.5), None, None)?;
        let a = self
            .out
            .forward(&a.as_dtype(n.dtype())?.reshape(&[b, h * w, c])?)?
            .reshape(shape)?;
        self.last.forward(&ops::add(y, a)?)
    }
}
