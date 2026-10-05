# pocket-tts, as Coupler ships it

Kyutai's **Pocket TTS** (github.com/kyutai-labs/pocket-tts, MIT) as
ported to Rust and Candle by the Pocket TTS contributors
(github.com/babybirdprd/pocket-tts, crate `pocket-tts` 0.6.2, MIT OR
Apache-2.0; its license file is `LICENSE-MIT`, the upstream text with
its holder named). Vendored into Coupler on 2026-10-03 because the
published crate can't ship in Coupler as it is.

## What Coupler changed

- **No downloader.** `weights.rs` and `hf-hub` are gone: the crate
  fetched weights from Hugging Face whenever a path began `hf://`, and
  Coupler talks to nothing but CoffeeMUD (Coupler's CLAUDE.md, rule 2).
  `TTSModel::load` takes the config, a local weights file and the
  tokenizer's bytes.
- **No Intel MKL.** `intel-mkl-src` was a dependency on every desktop
  target; MKL is under Intel's own license, not an open-source one.
  Candle runs on its own CPU kernels (gemm).
- **No voice cloning.** The Mimi encoder, its transformer and the
  downsampler are removed (`models/mimi.rs`), and with them every way
  to make a voice from a recording (`audio.rs`, `get_voice_state` and
  friends). Kyutai's terms forbid cloning without consent; Coupler ships
  presets only. Coupler's weights file leaves those tensors out.
- **Saved voices.** `get_voice_state_from_kv_bytes` loads Kyutai's
  exported voice states (`embeddings_v3/*.safetensors`, a key and value
  cache per FlowLM layer), so all the permissively licensed presets can
  ship, not only the eight older prompt files.
- **Removed:** the wasm bindings, the quantization experiment, the
  commented-out parallel decoder, the config-file search, benches,
  examples and tests that needed downloads; `tokenizers` without its
  default features (Oniguruma, a progress bar).

Everything else (the models, the sampling, the text preparation) is the
port as published.
