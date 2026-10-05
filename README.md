# Coupler

A dedicated client for [CoffeeMUD](http://www.coffeemud.net/), and for
nothing else. Coupler is built to be the easiest way into CoffeeMUD for
blind and visually impaired players, and a good one for everyone.

Named for the acoustic coupler, the cradle a phone handset sat in to
carry a modem's data. Part of the ansiapps family. Free and open
source under the Apache License 2.0.

> Coupler is in development. It runs on macOS first; other platforms
> come later.

## What it does

- **Three ways to play.** Terminal (the game's own screen, 80 columns),
  Immersive (sound first and self-voiced: cues, a narrator, a voice for
  every character) and Workshop (everything at once, arranged your way).
- **Accessible first.** Everything works from the keyboard and with
  VoiceOver. A picture always has the same facts in words beside it.
  Speech is kept short: what can be a sound is one. Review mode, sound
  captions, readable colors and big text are built in.
- **Hears the game.** CoffeeMUD's GMCP and MSDP: vitals, fights,
  who's talking and where you are, said or played rather than read off
  the screen.
- **An auto-mapper** from the game's room data, with directions in words
  and walking a route one step at a time.
- **Hooks and assets.** Sounds, music, background ambience and ANSI art
  on anything the game reports. Coupler's own synthesizer plays MIDI
  without a SoundFont, and it can compose music and paint pictures from
  your words, all on your computer.
- **Room pictures** drawn in ANSI by Coupler's own painter.
- **A journal** of everything said to you, kept on your computer.

Coupler talks to CoffeeMUD and to nothing else: no accounts, no
telemetry, no server of ours. Nothing about you or your play leaves
your computer except what you send the game. The one exception is
yours to turn on: checking for updates (gear menu) asks GitHub which
Coupler release is the latest, sending nothing but Coupler's version.

## Building

You need Node.js, Rust (stable), CMake, and on macOS the Xcode command
line tools.

```sh
npm install
scripts/pocket-tts-fetch.py      # once: the bundled voices' model, ~250 MB
npm run tauri dev                # a dev build (Vite on port 1460)
scripts/build-release.sh --bundles app   # a release Coupler.app
```

Always build releases with `scripts/build-release.sh`: it keeps your
machine's paths out of the app.

The web build (`npm run build:web`) is a slimmer Coupler that runs in a
browser.

Tests: `cargo test --manifest-path src-tauri/Cargo.toml`,
`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets` and
`npx tsc --noEmit`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security problems:
[SECURITY.md](SECURITY.md). `CLAUDE.md` is the full set of operating
rules, written for Claude Code and for anyone working on Coupler.

## License

Apache License 2.0: see [LICENSE](LICENSE) and [NOTICE](NOTICE). Every
third-party license Coupler ships is in the app, under About →
Open-Source Licenses.

Coupler's protocol handling follows Sip, CoffeeMUD's own client by Bo
Zimmerman. CoffeeMUD is by Bo Zimmerman and its contributors.
