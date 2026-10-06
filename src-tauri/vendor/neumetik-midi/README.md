# Neumetik MIDI (Coupler's copy)

Neumetik MIDI is ansiapps' MIDI synthesizer: General MIDI and Roland GS
music played without a SoundFont or samples. Three small engines
(subtractive with supersaw unison and sync, two-operator FM,
Karplus-Strong plucked strings), an SVF filter, envelopes, an LFO,
chorus and reverb, every drum a recipe. Standard library only.

## License

MIT (`LICENSE` here), copyright 2026 ansiapps: its own license, not
Coupler's, and compatible with it.

## Where it's changed

The original lives in ansiapps' Neumetik repository (`midi/`, the
`neumetik-midi` crate), which also has its C API, WebAssembly build,
bindings and demo page. Make changes there and copy them here with
`scripts/neumetik-sync.sh`, so the two never drift: a change made only
here is overwritten by the next sync. Coupler doesn't take outside
contributions to this folder.

## Tests

`cargo test` here. Two ignored tests help while tuning:
`OUT=demo.wav cargo test --release neumetik_demo -- --ignored` writes a
tour of the banks, and `cargo test --release neumetik_speed -- --ignored
--nocapture` prints how much faster than real time it renders.
