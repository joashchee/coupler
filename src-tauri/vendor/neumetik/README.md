# Neumetik (Coupler's copy)

Neumetik is ansiapps' synthesizer: General MIDI and Roland GS music
played without a SoundFont or samples. Three small engines
(subtractive with supersaw unison and sync, two-operator FM,
Karplus-Strong plucked strings), an SVF filter, envelopes, an LFO,
chorus and reverb, every drum a recipe. Standard library only.

## License

This copy is part of Coupler and, like the rest of Coupler, is licensed
under the Apache License, Version 2.0 (Coupler's `LICENSE`).
Copyright 2026 ansiapps.

## Where it's changed

The original lives in ansiapps' private Neumetik repository, which is
also licensed to ansiapps' other apps. Make changes there and copy them
here with `scripts/neumetik-sync.sh`, so the two never drift: a change
made only here is overwritten by the next sync. Coupler doesn't take
outside contributions to this folder.

## Tests

`cargo test` here. Two ignored tests help while tuning:
`OUT=demo.wav cargo test --release neumetik_demo -- --ignored` writes a
tour of the banks, and `cargo test --release neumetik_speed -- --ignored
--nocapture` prints how much faster than real time it renders.
