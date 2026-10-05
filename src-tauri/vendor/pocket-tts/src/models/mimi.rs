use crate::ModelState;
use crate::models::seanet::SEANetDecoder;
use crate::models::transformer::ProjectedTransformer;
use crate::modules::conv::ConvTrUpsample1d;
use candle_core::{Result, Tensor};
use candle_nn::{Conv1d, Conv1dConfig, Module, VarBuilder};

#[derive(Clone)]
pub struct Quantizer {
    output_proj: Conv1d,
}

impl Quantizer {
    pub fn new(dimension: usize, output_dimension: usize, vb: VarBuilder) -> Result<Self> {
        let config = Conv1dConfig {
            groups: 1,
            padding: 0,
            stride: 1,
            dilation: 1,
            ..Default::default()
        };
        let output_proj = candle_nn::conv1d_no_bias(
            dimension,
            output_dimension,
            1,
            config,
            vb.pp("output_proj"),
        )?;
        Ok(Self { output_proj })
    }

    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        // x is [B, C, T]
        // Conv1d expects [B, C, T] and returns [B, C_out, T]
        self.output_proj.forward(x)
    }
}

/// Mimi, decoding only. Coupler (vendor/README.md) removed the encoder,
/// its transformer and the downsampler: they turn recorded speech into a
/// voice, which is voice cloning, and Coupler offers presets only.
#[derive(Clone)]
pub struct MimiModel {
    pub decoder: SEANetDecoder,
    pub decoder_transformer: ProjectedTransformer,
    pub quantizer: Quantizer,
    pub upsample: Option<ConvTrUpsample1d>,
    pub frame_rate: f64,
    pub sample_rate: usize,
    pub channels: usize,
    pub dimension: usize,
}

impl MimiModel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        decoder: SEANetDecoder,
        decoder_transformer: ProjectedTransformer,
        frame_rate: f64,
        encoder_frame_rate: f64,
        sample_rate: usize,
        channels: usize,
        dimension: usize,        // The quantizer input dimension (32)
        output_dimension: usize, // The decoder input dimension (512)
        name: &str,
        vb: VarBuilder,
    ) -> Result<Self> {
        let quantizer = Quantizer::new(dimension, output_dimension, vb.pp("quantizer"))?;

        let upsample = if encoder_frame_rate != frame_rate {
            let stride = (encoder_frame_rate / frame_rate) as usize;
            Some(ConvTrUpsample1d::new(
                stride,
                output_dimension,
                &format!("{}.upsample", name),
                vb.pp("upsample"),
            )?)
        } else {
            None
        };

        Ok(Self {
            decoder,
            decoder_transformer,
            quantizer,
            upsample,
            frame_rate,
            sample_rate,
            channels,
            dimension,
        })
    }

    pub fn frame_size(&self) -> usize {
        (self.sample_rate as f64 / self.frame_rate) as usize
    }

    pub fn decode_from_latent(
        &self,
        latent: &Tensor,
        model_state: &mut ModelState,
        step: usize,
    ) -> Result<Tensor> {
        let mut emb = latent.clone();
        if let Some(up) = &self.upsample {
            emb = up.forward(&emb, model_state, step)?;
        }
        let mut embs = self.decoder_transformer.forward(&emb, model_state, step)?;
        emb = embs.remove(0);
        let out = self.decoder.forward(&emb, model_state, step)?;
        Ok(out)
    }
    pub fn quantize(&self, x: &Tensor) -> Result<Tensor> {
        self.quantizer.forward(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device};
    use candle_nn::VarBuilder;

    #[test]
    fn test_mimi_shapes() -> Result<()> {
        let device = Device::Cpu;
        let vb = VarBuilder::zeros(DType::F32, &device);
        let decoder = SEANetDecoder::new(1, 128, 32, 1, &[2, 2], 7, 7, 3, 2, "constant", 2, "decoder", vb.pp("decoder"))?;
        let decoder_transformer = ProjectedTransformer::new(128, vec![128], 128, 4, 1, 0.1, 10, 10000.0, 512, "dec_tr", vb.pp("dec_tr"))?;
        let mimi = MimiModel::new(decoder, decoder_transformer, 12.5, 50.0, 16000, 1, 128, 512, "mimi", vb.pp("mimi"))?;
        assert_eq!(mimi.frame_size(), 1280);
        Ok(())
    }
}
