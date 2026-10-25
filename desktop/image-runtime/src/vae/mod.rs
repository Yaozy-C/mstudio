//! SeedVR2 still-frame encoder/decoder, using its original weights and 2D convolutions.
mod layers;
#[cfg(test)]
mod tests;
use anyhow::Result;
use layers::{Conv, Mid, Norm, Residual, silu};
use mlx_gen::weights::Weights;
use mlx_rs::{Array, ops};

#[cfg(test)]
pub struct Vae {
    pub encoder: Encoder,
    pub decoder: Decoder,
}
#[cfg(test)]
impl Vae {
    pub fn load(w: &Weights) -> Result<Self> {
        Ok(Self {
            encoder: Encoder::load(w)?,
            decoder: Decoder::load(w)?,
        })
    }
}

struct Down {
    blocks: Vec<Residual>,
    sample: Option<Conv>,
}
impl Down {
    fn load(w: &Weights, index: usize) -> Result<Self> {
        let key = format!("encoder.down_blocks.{index}");
        Ok(Self {
            blocks: (0..2)
                .map(|i| Residual::load(w, &format!("{key}.resnets.{i}")))
                .collect::<Result<_>>()?,
            sample: if index < 3 {
                Some(Conv::load(w, &format!("{key}.downsamplers.0.conv"), 2, 0)?)
            } else {
                None
            },
        })
    }
    fn forward(&self, x: &Array) -> Result<Array> {
        let mut x = x.clone();
        for block in &self.blocks {
            x = block.forward(&x)?;
        }
        if let Some(c) = &self.sample {
            x = c.forward(&ops::pad(
                &x,
                &[(0, 0), (0, 1), (0, 1), (0, 0)][..],
                None,
                None,
            )?)?;
        }
        Ok(x)
    }
}

pub struct Encoder {
    input: Conv,
    down: Vec<Down>,
    mid: Mid,
    norm: Norm,
    out: Conv,
}
impl Encoder {
    pub(crate) fn load(w: &Weights) -> Result<Self> {
        Ok(Self {
            input: Conv::load(w, "encoder.conv_in", 1, 1)?,
            down: (0..4).map(|i| Down::load(w, i)).collect::<Result<_>>()?,
            mid: Mid::load(w, "encoder.mid_block")?,
            norm: Norm::load(w, "encoder.conv_norm_out")?,
            out: Conv::load(w, "encoder.conv_out", 1, 1)?,
        })
    }
    /// NHWC pixels in [-1,1] -> NHWC mean latent, scaled by 0.9152.
    pub fn forward(&self, x: &Array) -> Result<Array> {
        let mut x = self.input.forward(x)?;
        for down in &self.down {
            x = down.forward(&x)?;
        }
        x = self.mid.forward(&x)?;
        let x = self.out.forward(&silu(&self.norm.forward(&x)?)?)?;
        Ok(ops::multiply(
            &ops::split(&x, 2, 3)?[0],
            Array::from_f32(0.9152),
        )?)
    }
}

struct Up {
    blocks: Vec<Residual>,
    sample: Option<(Conv, Conv, i32)>,
}
impl Up {
    fn load(w: &Weights, index: usize) -> Result<Self> {
        let key = format!("decoder.up_blocks.{index}");
        let sample = if index < 3 {
            Some((
                Conv::load(w, &format!("{key}.upsamplers.0.upscale_conv"), 1, 0)?,
                Conv::load(w, &format!("{key}.upsamplers.0.conv"), 1, 1)?,
                if index < 2 { 2 } else { 1 },
            ))
        } else {
            None
        };
        Ok(Self {
            blocks: (0..3)
                .map(|i| Residual::load(w, &format!("{key}.resnets.{i}")))
                .collect::<Result<_>>()?,
            sample,
        })
    }
    fn forward(&self, x: &Array) -> Result<Array> {
        let mut x = x.clone();
        for block in &self.blocks {
            x = block.forward(&x)?;
        }
        if let Some((expand, conv, temporal)) = &self.sample {
            let shape = x.shape();
            let (b, h, w, c) = (shape[0], shape[1], shape[2], shape[3]);
            let expanded = expand
                .forward(&x)?
                .reshape(&[b, h, w, 2, 2, *temporal, c])?;
            let first = expanded.take_axis(Array::from_int(0), 5)?;
            x = conv.forward(&first.transpose_axes(&[0, 1, 3, 2, 4, 5])?.reshape(&[
                b,
                h * 2,
                w * 2,
                c,
            ])?)?;
        }
        Ok(x)
    }
}

pub struct Decoder {
    input: Conv,
    mid: Mid,
    up: Vec<Up>,
    norm: Norm,
    out: Conv,
}
impl Decoder {
    pub(crate) fn load(w: &Weights) -> Result<Self> {
        Ok(Self {
            input: Conv::load(w, "decoder.conv_in", 1, 1)?,
            mid: Mid::load(w, "decoder.mid_block")?,
            up: (0..4).map(|i| Up::load(w, i)).collect::<Result<_>>()?,
            norm: Norm::load(w, "decoder.conv_norm_out")?,
            out: Conv::load(w, "decoder.conv_out", 1, 1)?,
        })
    }
    /// NHWC scaled latent -> NHWC decoded pixels in [-1,1].
    pub fn forward(&self, z: &Array) -> Result<Array> {
        let z = ops::multiply(z, Array::from_f32(1.0 / 0.9152))?;
        let mut x = self.mid.forward(&self.input.forward(&z)?)?;
        for up in &self.up {
            x = up.forward(&x)?;
        }
        self.out.forward(&silu(&self.norm.forward(&x)?)?)
    }
}
