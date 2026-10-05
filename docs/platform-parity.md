# Platform parity

Every macOS-specific mechanism Coupler uses, and what Windows and Linux
would need. Windows and Linux builds come after macOS covers the
milestone. Unresearched beyond naming the mechanism.

| Feature | macOS | Windows | Linux |
|---|---|---|---|
| Release build | `scripts/build-release.sh` on the maintainer's Mac | GitHub Actions, `.github/workflows/windows.yml` (NSIS and MSI, unsigned) | GitHub Actions, `.github/workflows/linux.yml` (.deb and .rpm on Ubuntu 22.04; no AppImage, which would bundle LGPL WebKitGTK) |
| Install location for local release builds | `~/Applications/Coupler.app` via `ditto` (`scripts/build-release.sh`) | n/a (installer) | n/a (package) |
| Full screen | Native full screen (its own Space) through Tauri's `set_fullscreen`; tao makes the fixed-size window resizable for the switch and restores it after. Key ⌃⌘F | Borderless full screen through the same call. Key should be F11 or Alt+Enter | Same call; depends on the window manager. Key F11 |
| Immersive's voice | WebKit's `speechSynthesis` (the system voices, AVSpeechSynthesizer); `lib/voice.ts`. Planned: `AVSpeechSynthesizer` from Rust, its audio through the mixer | WebView2's `speechSynthesis` (SAPI/OneCore voices); natively SAPI or OneCore | WebKitGTK may have no `speechSynthesis`: Speech Dispatcher (check its client library's license: LGPL rules it out of the binary) or bundled Flite (BSD-style). Not eSpeak NG (GPL-3) |
| Characters' voices in Flite (`synth.rs`) | Pure Rust (`flite-rs`), played through Web Audio: nothing macOS-specific | Same | Same; and Flite stands in for a missing `speechSynthesis` there |
| Characters' voices in Pocket TTS (`synth.rs`, `vendor/pocket-tts`) | Candle on the CPU, pure Rust, the files in `Coupler.app/Contents/Resources/pocket-tts` (`resource_dir`) | Same; the files beside the executable in the installer's resources. CPU speed varies more: check a line still comes in time | Same; check the package puts the resources where `resource_dir` looks |
| Characters' voices by gender | `lib/voice.ts` `GENDERS`: macOS's voice names (Samantha, Daniel, …) marked feminine or masculine, and its novelty voices (Zarvox, …) left out of Automatic; `speechSynthesis` says nothing of a voice's gender | Windows' voice names (Zira, David, …) need their own entries | Whatever engine speaks: its voices' names, or its own variants (Flite's voices are named by speaker) |
| The game's talk in bundled voices (planned) | Pocket TTS in Rust (Candle, Metal) | Same, CPU | Same, CPU |
| The voice cache's free-space check (`voicecache.rs` `free_space`) | `statvfs` (libc) on the app-data volume; `f_bavail` × `f_frsize` | `GetDiskFreeSpaceExW` (not built: until then the cache keeps to its 1 GB cap alone) | `statvfs`, as on macOS (built, untested) |
| Fixed window size | `resizable: false` greys out the green button; a window taller than the screen is shrunk by the system, and the stage scales down to fit | Check 1366×768 with the taskbar: the window is taller than the work area | Check tiling window managers, which ignore fixed sizes (the stage scales to whatever it's given) |

The connection itself (`std::net::TcpStream`, `flate2`) is the same on
every platform. Planned, add a row when each lands: saved passwords
(macOS Keychain; Windows Credential Manager; Linux Secret Service via
libsecret/D-Bus), sound playback for MSP/Client.Media, notifications for
tells.
