//! Main TTSModel struct - orchestrates the TTS pipeline
//!
//! This is the high-level API for text-to-speech generation,
//! matching Python's `pocket_tts/models/tts_model.py`.

use crate::ModelState;
use crate::conditioners::text::LUTConditioner;
use crate::config::{Config, defaults};
use crate::models::flow_lm::FlowLMModel;
use crate::models::mimi::MimiModel;
use crate::models::seanet::SEANetDecoder;
use crate::models::transformer::{ProjectedTransformer, StreamingTransformer};
use crate::modules::mlp::SimpleMLPAdaLN;
use crate::voice_state::{increment_steps, init_states};

use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;

/// Main TTS model that orchestrates the entire pipeline
#[derive(Clone)]
pub struct TTSModel {
    /// Flow language model for latent generation
    pub flow_lm: FlowLMModel,
    /// Mimi neural audio codec
    pub mimi: MimiModel,
    /// Text conditioner (tokenizer + embeddings)
    pub conditioner: LUTConditioner,
    /// Generation temperature
    pub temp: f32,
    /// Number of LSD decode steps
    pub lsd_decode_steps: usize,
    /// End-of-sequence threshold
    pub eos_threshold: f32,
    pub noise_clamp: Option<f32>,
    /// Sample rate
    pub sample_rate: usize,
    /// Model dimension
    pub dim: usize,
    /// Latent dimension
    pub ldim: usize,
    /// Device
    pub device: Device,
}

impl TTSModel {
    /// Loads the model from local files only (Coupler: nothing is ever
    /// downloaded). `weights` is memory-mapped; any float type in it is
    /// read as f32.
    pub fn load(config_yaml: &[u8], weights: &std::path::Path, tokenizer_json: &[u8]) -> Result<Self> {
        let config: Config = serde_yaml::from_slice(config_yaml)?;
        let device = Device::Cpu;
        // SAFETY: the file is the app's own, read-only, and not changed while mapped.
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], DType::F32, &device)? };
        let conditioner = LUTConditioner::new_from_bytes(
            config.flow_lm.lookup_table.n_bins,
            tokenizer_json,
            config.flow_lm.lookup_table.dim,
            config.flow_lm.transformer.d_model,
            vb.pp("flow_lm.conditioner"),
        )?;
        Self::from_config_and_vb(
            config,
            defaults::TEMPERATURE,
            defaults::LSD_DECODE_STEPS,
            defaults::EOS_THRESHOLD,
            None,
            conditioner,
            vb,
        )
    }

    /// Internal helper to build model from config and VarBuilder
    fn from_config_and_vb(
        config: Config,
        temp: f32,
        lsd_decode_steps: usize,
        eos_threshold: f32,
        noise_clamp: Option<f32>,
        conditioner: LUTConditioner,
        vb: VarBuilder,
    ) -> Result<Self> {
        let device = vb.device().clone();

        // Build FlowLM components
        let dim = config.flow_lm.transformer.d_model;
        let ldim = config.mimi.quantizer.dimension;
        let hidden_dim = dim * config.flow_lm.transformer.hidden_scale;

        // SimpleMLPAdaLN::new(in_channels, model_channels, out_channels, cond_channels, num_res_blocks, num_time_conds, max_period, vb)
        let flow_net = SimpleMLPAdaLN::new(
            ldim,                      // in_channels (input is latent dim)
            config.flow_lm.flow.dim,   // model_channels
            ldim,                      // out_channels (output is also latent dim)
            dim,                       // cond_channels (conditioning from transformer)
            config.flow_lm.flow.depth, // num_res_blocks
            2,                         // num_time_conds (s and t)
            config.flow_lm.transformer.max_period as f32,
            vb.pp("flow_lm.flow_net"),
        )?;

        // StreamingTransformer::new(d_model, num_heads, num_layers, layer_scale, dim_feedforward, context, max_period, kind, name, vb)
        let transformer = StreamingTransformer::new(
            dim,
            config.flow_lm.transformer.num_heads,
            config.flow_lm.transformer.num_layers,
            None,       // layer_scale
            hidden_dim, // dim_feedforward
            None,       // context (causal)
            config.flow_lm.transformer.max_period as f32,
            "kv",
            "flow_lm.transformer",
            vb.pp("flow_lm.transformer"),
        )?;

        let mut flow_lm = FlowLMModel::new(flow_net, transformer, ldim, dim, vb.pp("flow_lm"))?;
        flow_lm.noise_clamp = noise_clamp;

        // Build Mimi components
        let seanet_cfg = &config.mimi.seanet;
        let decoder = SEANetDecoder::new(
            seanet_cfg.channels,
            seanet_cfg.dimension,
            seanet_cfg.n_filters,
            seanet_cfg.n_residual_layers,
            &seanet_cfg.ratios,
            seanet_cfg.kernel_size,
            seanet_cfg.residual_kernel_size,
            seanet_cfg.last_kernel_size,
            seanet_cfg.dilation_base,
            &seanet_cfg.pad_mode,
            seanet_cfg.compress,
            "mimi.decoder",
            vb.pp("mimi.decoder"),
        )?;

        let mimi_tr_cfg = &config.mimi.transformer;
        // ProjectedTransformer::new(input_dimension, output_dimensions, d_model, num_heads, num_layers, layer_scale, context, max_period, dim_feedforward, name, vb)
        let decoder_transformer = ProjectedTransformer::new(
            mimi_tr_cfg.input_dimension,
            mimi_tr_cfg.output_dimensions.clone(),
            mimi_tr_cfg.d_model,
            mimi_tr_cfg.num_heads,
            mimi_tr_cfg.num_layers,
            mimi_tr_cfg.layer_scale as f32,
            mimi_tr_cfg.context,
            mimi_tr_cfg.max_period as f32,
            mimi_tr_cfg.dim_feedforward,
            "mimi.decoder_transformer",
            vb.pp("mimi.decoder_transformer"),
        )?;

        // Calculate encoder frame rate from SEANet ratios
        let hop_length: usize = seanet_cfg.ratios.iter().product();
        let encoder_frame_rate = config.mimi.sample_rate as f64 / hop_length as f64;

        let mimi = MimiModel::new(
            decoder,
            decoder_transformer,
            config.mimi.frame_rate,
            encoder_frame_rate,
            config.mimi.sample_rate,
            config.mimi.channels,
            config.mimi.quantizer.dimension,
            config.mimi.quantizer.output_dimension,
            "mimi",
            vb.pp("mimi"),
        )?;

        Ok(Self {
            flow_lm,
            mimi,
            conditioner,
            temp,
            lsd_decode_steps,
            eos_threshold,
            noise_clamp,
            sample_rate: config.mimi.sample_rate,
            dim,
            ldim,
            device,
        })
    }

    /// Create voice state from pre-calculated latent prompt bytes (.safetensors)
    pub fn get_voice_state_from_prompt_bytes(&self, bytes: &[u8]) -> Result<ModelState> {
        let tensors = candle_core::safetensors::load_buffer(bytes, &self.device)?;
        let prompt = tensors
            .get("audio_prompt")
            .ok_or_else(|| anyhow::anyhow!("'audio_prompt' not found in safetensors bytes"))?;

        self.get_voice_state_from_prompt_tensor(prompt)
    }

    /// Create voice state from a pre-calculated latent prompt tensor
    pub fn get_voice_state_from_prompt_tensor(&self, prompt: &Tensor) -> Result<ModelState> {
        // Ensure prompt tensor is on the same device as the model (fixes Metal device mismatch)
        let prompt = if prompt.device().same_device(&self.device) {
            prompt.clone()
        } else {
            prompt.to_device(&self.device)?
        };

        let mut flow_state = init_states(1, 1000);
        self.run_flow_lm_prompt(&prompt, &mut flow_state)?;
        Ok(flow_state)
    }

    /// A voice state saved after prompting: Kyutai's `export_model_state`
    /// files (`embeddings_v3/`), each FlowLM attention layer's key and
    /// value cache as `transformer.layers.N.self_attn/cache`, shaped
    /// [k or v, batch, frames, heads, head dim], and its `/offset`. Added
    /// by Coupler: it gives the same state `run_flow_lm_prompt` would.
    pub fn get_voice_state_from_kv_bytes(&self, bytes: &[u8]) -> Result<ModelState> {
        use crate::voice_state::{ATTN_K_BUF_KEY, ATTN_V_BUF_KEY, AttentionCursor, write_attention_cursor};
        let tensors = candle_core::safetensors::load_buffer(bytes, &self.device)?;
        let mut state = init_states(1, 1000);
        let mut layer = 0;
        while let Some(cache) = tensors.get(&format!("transformer.layers.{layer}.self_attn/cache")) {
            let cache = cache.to_dtype(DType::F32)?;
            let (two, _b, frames, _h, _d) = cache.dims5()?;
            if two != 2 {
                anyhow::bail!("layer {layer}'s cache isn't a key and a value");
            }
            let k = cache.get(0)?.transpose(1, 2)?.contiguous()?;
            let v = cache.get(1)?.transpose(1, 2)?.contiguous()?;
            let mut module = std::collections::HashMap::new();
            module.insert(ATTN_K_BUF_KEY.to_string(), k);
            module.insert(ATTN_V_BUF_KEY.to_string(), v);
            write_attention_cursor(&mut module, AttentionCursor { pos: frames, len: frames, head: 0 }, &self.device)?;
            state.insert(format!("flow_lm.transformer.layers.{layer}.self_attn"), module);
            layer += 1;
        }
        if layer == 0 {
            anyhow::bail!("no attention cache in the voice file");
        }
        Ok(state)
    }

    /// Run flow LM with audio conditioning (used during prompting)
    fn run_flow_lm_prompt(&self, conditioning: &Tensor, state: &mut ModelState) -> Result<()> {
        // Empty text tokens and backbone input
        let empty_text = Tensor::zeros((1, 0), DType::I64, &self.device)?;
        let text_embeddings = self.conditioner.forward(&empty_text)?;

        // Concatenate text embeddings and audio conditioning
        // Match Python/reference order: audio conditioning comes before text embeddings.
        let input = Tensor::cat(&[conditioning, &text_embeddings], 1)?;

        // Run through transformer (no generation, just prompting)
        // With custom SDPA, this is now memory efficient
        let _ = self.flow_lm.transformer.forward(&input, state, 0)?;

        // Increment FlowLM state after prompting (critical for RoPE positioning)
        // Python: increment_steps(self.flow_lm, model_state, increment=audio_conditioning.shape[1])
        let increment_by = conditioning.dims()[1];
        increment_steps(state, "offset", increment_by);

        Ok(())
    }

    /// Split text into optimal chunks for generation, matching Python's logic exactly.
    /// Uses actual tokenization to ensure chunks never exceed MAX_TOKENS_PER_CHUNK (50).
    /// This prevents O(N²) attention complexity for long texts.
    pub fn split_into_best_sentences(&self, text: &str) -> Vec<String> {
        const MAX_TOKENS_PER_CHUNK: usize = 50;

        let prepared_text = prepare_text_prompt(text);

        // 1. Initial split by punctuation to respect sentence boundaries
        let raw_sentences: Vec<&str> = prepared_text
            .split_inclusive(&['.', '!', '?', ';', ':'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if raw_sentences.is_empty() {
            return vec![prepared_text];
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_token_count = 0;

        for sentence in raw_sentences {
            let sentence_tokens = self
                .conditioner
                .count_tokens(sentence)
                .unwrap_or(MAX_TOKENS_PER_CHUNK);

            // If a single sentence exceeds max tokens, split it by words
            if sentence_tokens > MAX_TOKENS_PER_CHUNK {
                // Flush pending chunk first
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                    current_chunk = String::new();
                    current_token_count = 0;
                }

                // Split long sentence using word-batch estimation (~1.3 tokens per word average)
                // This avoids calling count_tokens for every word (expensive!)
                let words: Vec<&str> = sentence.split_whitespace().collect();
                const WORDS_PER_BATCH: usize = 35; // ~45 tokens, safe margin under 50

                for word_batch in words.chunks(WORDS_PER_BATCH) {
                    let chunk_str = word_batch.join(" ");
                    // Verify this batch is actually under limit (should almost always pass)
                    let actual_tokens = self
                        .conditioner
                        .count_tokens(&chunk_str)
                        .unwrap_or(MAX_TOKENS_PER_CHUNK);

                    if actual_tokens <= MAX_TOKENS_PER_CHUNK {
                        chunks.push(chunk_str);
                    } else {
                        // Rare case: batch still too big, split in half recursively
                        let mid = word_batch.len() / 2;
                        chunks.push(word_batch[..mid].join(" "));
                        chunks.push(word_batch[mid..].join(" "));
                    }
                }
                continue;
            }

            // Normal accumulation logic
            if current_chunk.is_empty() {
                current_chunk = sentence.to_string();
                current_token_count = sentence_tokens;
            } else if current_token_count + sentence_tokens > MAX_TOKENS_PER_CHUNK {
                chunks.push(current_chunk);
                current_chunk = sentence.to_string();
                current_token_count = sentence_tokens;
            } else {
                current_chunk.push(' ');
                current_chunk.push_str(sentence);
                current_token_count += sentence_tokens;
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    /// Generate audio from text with voice state
    pub fn generate(&self, text: &str, voice_state: &ModelState) -> Result<Tensor> {
        let mut audio_chunks = Vec::new();

        for chunk in self.generate_stream(text, voice_state) {
            audio_chunks.push(chunk?);
        }

        // Concatenate all audio chunks
        if audio_chunks.is_empty() {
            anyhow::bail!("No audio generated");
        }
        let audio = Tensor::cat(&audio_chunks, 2)?;
        // Remove batch dimension
        let audio = audio.squeeze(0)?;

        Ok(audio)
    }


    /// Generate audio from text with pause handling
    ///
    /// This method parses pause markers in the text and inserts silence
    /// at appropriate positions. Supports:
    /// - Explicit pauses: `[pause:500ms]` or `[pause:1s]`
    /// - Natural pauses from punctuation are handled during generation
    ///
    /// # Example
    /// ```ignore
    /// let audio = model.generate_with_pauses("Hello... [pause:500ms] world", &voice_state)?;
    /// ```
    pub fn generate_with_pauses(&self, text: &str, voice_state: &ModelState) -> Result<Tensor> {
        let mut audio_chunks = Vec::new();

        for chunk in self.generate_stream_long(text, voice_state) {
            audio_chunks.push(chunk?);
        }

        // Concatenate all audio chunks
        if audio_chunks.is_empty() {
            anyhow::bail!("No audio generated");
        }
        let audio = Tensor::cat(&audio_chunks, 2)?;
        // Remove batch dimension
        let audio = audio.squeeze(0)?;

        Ok(audio)
    }

    /// Generate audio stream from text with voice state
    ///
    /// Returns an iterator that yields audio chunks (one per Mimi frame).
    /// Generate audio stream from text with voice state
    ///
    /// Returns an iterator that yields audio chunks (one per Mimi frame).
    ///
    /// This method splits the text into optimal sentences and generates each independently,
    /// matching Python's behavior to maintain O(N) complexity for long texts.
    pub fn generate_stream<'a, 'b, 'c>(
        &'a self,
        text: &'b str,
        voice_state: &'c ModelState,
    ) -> Box<dyn Iterator<Item = Result<Tensor>> + 'a> {
        // Split text into chunks to avoid quadratic complexity scaling
        let chunks = self.split_into_best_sentences(text);

        // Clone voice state so the iterator owns a copy, untied from lifetime 'c
        let voice_state_owned = voice_state.clone();

        // Create an iterator that processes each chunk sequentially
        let iterator = chunks.into_iter().flat_map(move |chunk_text| {
            // We need to return an iterator for each chunk.
            // We pass a reference to the owned voice state captured by the closure.
            self.generate_stream_segment(chunk_text, &voice_state_owned)
        });

        Box::new(iterator)
    }

    /// Generate audio stream from text with voice state, returning an owned iterator.
    ///
    /// This is useful for WASM bindings where the iterator must be 'static.
    pub fn generate_stream_owned(
        &self,
        text: &str,
        voice_state: &ModelState,
    ) -> Box<dyn Iterator<Item = Result<Tensor>> + 'static> {
        let model = self.clone();
        let voice_state_owned = voice_state.clone();
        let chunks = model.split_into_best_sentences(text);

        let iterator = chunks.into_iter().flat_map(move |chunk_text| {
            model.generate_stream_segment(chunk_text, &voice_state_owned)
        });

        Box::new(iterator)
    }

    /// Internal helper to generate a single segment (short text) matching Python's _generate
    fn generate_stream_segment(
        &self,
        text: String,
        voice_state: &ModelState,
    ) -> Box<dyn Iterator<Item = Result<Tensor>>> {
        let mut state = voice_state.clone();
        let mut mimi_state = init_states(1, 1000);

        // Prepare text
        let prepared_text = prepare_text_prompt(&text);

        // Error handling for preparation failures inside the iterator
        let tokens = match self.conditioner.prepare(&prepared_text, &self.device) {
            Ok(t) => t,
            Err(e) => return Box::new(std::iter::once(Err(e))),
        };

        let text_embeddings = match self.conditioner.forward(&tokens) {
            Ok(e) => e,
            Err(e) => return Box::new(std::iter::once(Err(e))),
        };

        // Initial text prompt
        if let Err(e) = self
            .flow_lm
            .transformer
            .forward(&text_embeddings, &mut state, 0)
        {
            return Box::new(std::iter::once(Err(anyhow::Error::from(e))));
        }

        // Removed redundant increment_steps("offset") - handled internally by RoPE/Attention with current_end_len

        let max_gen_len = (prepared_text.split_whitespace().count() + 2) * 13;
        let frames_after_eos = estimate_frames_after_eos(&text);

        let mut backbone_input = match self.flow_lm.bos_emb.clone().reshape((1, 1, self.ldim)) {
            Ok(t) => t,
            Err(e) => return Box::new(std::iter::once(Err(anyhow::Error::from(e)))),
        };

        let mut eos_step: Option<usize> = None;
        let mut finished = false;

        // We need to move 'self' (reference) and owned data into the closure
        // But 'self' is in `generate_stream` lifetime?
        // We clone needed cheap things or use references.
        // `flow_lm`, `mimi` are part of self.
        // The closure will borrow `self`.

        // To make the iterator valid 'static or bound to self, we use move.
        // But we need access to self inside.
        // We can clone `self` if cheap? No, TTSModel is large (holds models).
        // But TTSModel derives Clone! And models are wrappers around Arcs (Candle tensors/vars).
        // So cloning TTSModel is CHEAP (shallow copy of Arc pointers).
        let model = self.clone();

        // Pre-compute time embeddings for the entire segment to avoid re-computing every frame
        // Now returns a single batched Tensor [num_steps, channels]
        let time_embeddings = match model.flow_lm.flow_net.compute_time_embeddings(
            model.lsd_decode_steps,
            &model.device,
            DType::F32,
        ) {
            Ok(te) => te,
            Err(e) => return Box::new(std::iter::once(Err(anyhow::Error::from(e)))),
        };

        let empty_text_embeddings =
            Tensor::zeros((1, 0, model.dim), DType::F32, &model.device).unwrap();

        Box::new((0..max_gen_len).map_while(move |step| {
            if finished {
                return None;
            }

            // Text embeddings are already processed into state during initialization (line 752-757),
            // so we always pass empty text embeddings during autoregressive generation.
            // Passing text_embeddings again would cause duplicate/repeated speech.
            let text_tokens_to_pass = &empty_text_embeddings;

            let (next_latent, is_eos) = match tracing::info_span!("flow_lm.forward", step = step)
                .in_scope(|| {
                    model.flow_lm.forward(
                        &backbone_input,
                        text_tokens_to_pass,
                        &mut state,
                        &time_embeddings,
                        model.temp,
                        model.eos_threshold,
                        step,
                    )
                }) {
                Ok(res) => res,
                Err(e) => return Some(Err(anyhow::anyhow!(e))),
            };

            let audio_frame = match (|| -> Result<Tensor> {
                let next_latent_denorm = next_latent
                    .broadcast_mul(&model.flow_lm.emb_std)?
                    .broadcast_add(&model.flow_lm.emb_mean)?;

                let mimi_input = next_latent_denorm.unsqueeze(1)?.transpose(1, 2)?;
                let quantized = model.mimi.quantize(&mimi_input)?;
                let audio = tracing::info_span!("mimi.decode_from_latent", step = step)
                    .in_scope(|| {
                        model
                            .mimi
                            .decode_from_latent(&quantized, &mut mimi_state, step)
                    })
                    .map_err(|e| anyhow::anyhow!(e))?;

                // Removed redundant increment_steps("offset") for mimi

                Ok(audio)
            })() {
                Ok(frame) => frame,
                Err(e) => return Some(Err(e)),
            };

            if is_eos && eos_step.is_none() {
                eos_step = Some(step);
            }

            if let Some(e_step) = eos_step
                && step >= e_step + frames_after_eos
            {
                finished = true;
            }

            backbone_input = next_latent.unsqueeze(1).unwrap();

            // Removed redundant increment_steps("offset") for FlowLM - handled by attention state

            Some(Ok(audio_frame))
        }))
    }

    /// Generate audio stream from long text by segmenting it
    pub fn generate_stream_long<'a>(
        &'a self,
        text: &str,
        voice_state: &'a ModelState,
    ) -> impl Iterator<Item = Result<Tensor>> + 'a {
        use crate::pause::{parse_text_with_pauses, silence_samples};

        let parsed = parse_text_with_pauses(text);
        let mut segments = Vec::new();

        // Interleave text chunks and pauses
        let mut last_pos = 0;
        for pause in &parsed.pauses {
            if pause.position > last_pos {
                let text_seg = &parsed.clean_text[last_pos..pause.position];
                if !text_seg.trim().is_empty() {
                    segments.push(Segment::Text(text_seg.to_string()));
                }
            }
            segments.push(Segment::Pause(pause.duration_ms));

            // Explicit pauses were replaced by a single space in clean_text
            // Natural pauses (commas, ellipses) are still in clean_text
            if pause.original.starts_with("[pause:") {
                last_pos = pause.position + 1;
            } else {
                last_pos = pause.position + pause.original.len();
            }
        }
        if last_pos < parsed.clean_text.len() {
            let text_seg = &parsed.clean_text[last_pos..];
            if !text_seg.trim().is_empty() {
                segments.push(Segment::Text(text_seg.to_string()));
            }
        }

        let model = self;
        segments.into_iter().flat_map(move |seg| match seg {
            Segment::Text(s) => {
                let iter = model.generate_stream(&s, voice_state);
                Box::new(iter) as Box<dyn Iterator<Item = Result<Tensor>>>
            }
            Segment::Pause(ms) => {
                let n_samples = silence_samples(ms, model.sample_rate as u32);
                let silence_res = Tensor::zeros(
                    (1, model.mimi.channels, n_samples),
                    DType::F32,
                    &model.device,
                );
                Box::new(std::iter::once(silence_res.map_err(anyhow::Error::from)))
                    as Box<dyn Iterator<Item = Result<Tensor>>>
            }
        })
    }
    pub fn estimate_generation_steps(&self, text: &str) -> usize {
        let prepared = prepare_text_prompt(text);
        (prepared.split_whitespace().count() + 2) * 13
    }
}

/// Internal segment type for interleaving text and pauses
enum Segment {
    Text(String),
    Pause(u32),
}

/// Prepare text for generation, stripping pause markers for TTS processing
fn prepare_text_prompt(text: &str) -> String {
    // First strip any explicit pause markers
    let text = crate::pause::strip_pause_markers(text);

    let mut text = text.trim().to_string();
    if text.is_empty() {
        return ".".to_string(); // Or handle error
    }

    text = text.replace(['\n', '\r'], " ").replace("  ", " ");

    let word_count = text.split_whitespace().count();

    // Ensure first character is uppercase
    if let Some(first) = text.chars().next()
        && !first.is_uppercase()
    {
        text = format!("{}{}", first.to_uppercase(), &text[first.len_utf8()..]);
    }

    // Ensure ends with punctuation
    if let Some(last) = text.chars().last()
        && last.is_alphanumeric()
    {
        text.push('.');
    }

    // Python logic: prepend spaces if too short
    if word_count < 5 {
        text = format!("{}{}", " ".repeat(8), text);
    }

    text
}

/// Estimate frames after EOS based on text length
pub fn estimate_frames_after_eos(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    if word_count <= 4 {
        3 + 2 // prepare_text_prompt guess + 2
    } else {
        1 + 2 // prepare_text_prompt guess + 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_text_prompt() {
        // Short texts (<5 words) get 8 spaces prepended
        assert_eq!(prepare_text_prompt("hello world"), "        Hello world.");
        assert_eq!(prepare_text_prompt("Hello world."), "        Hello world.");
        assert_eq!(prepare_text_prompt("  hello  "), "        Hello.");
        // Long texts don't get spaces
        assert_eq!(
            prepare_text_prompt("one two three four five"),
            "One two three four five."
        );
    }

    #[test]
    fn test_prepare_text_prompt_strips_pause_markers() {
        // Pause markers should be stripped from text
        let result = prepare_text_prompt("Hello [pause:500ms] world");
        // The pause marker should be gone, replaced with space
        assert!(!result.contains("[pause:"));
        assert!(result.contains("Hello"));
        assert!(result.contains("world"));
    }

    #[test]
    fn test_prepare_text_prompt_handles_multiple_pauses() {
        let result = prepare_text_prompt("One [pause:100ms] two [pause:1s] three");
        assert!(!result.contains("[pause:"));
        assert!(result.contains("One"));
        assert!(result.contains("two"));
        assert!(result.contains("three"));
    }

    #[test]
    fn test_estimate_frames_after_eos() {
        // Short text (<= 4 words)
        assert_eq!(estimate_frames_after_eos("Hello world"), 5);
        // Longer text (> 4 words)
        assert_eq!(estimate_frames_after_eos("One two three four five"), 3);
    }

}
