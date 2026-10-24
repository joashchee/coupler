# Platform parity

Every macOS-specific mechanism Coupler uses, and what Windows and Linux
would need. Windows and Linux builds come after macOS covers the
milestone. Unresearched beyond naming the mechanism.

| Feature | macOS | Windows | Linux |
|---|---|---|---|
| Release build | `scripts/build-release.sh` on the maintainer's Mac | GitHub Actions, `.github/workflows/windows.yml` (NSIS and MSI, unsigned) | GitHub Actions, `.github/workflows/linux.yml` (.deb and .rpm on Ubuntu 22.04; no AppImage, which would bundle LGPL WebKitGTK) |
| Signing and notarization | Universal binary (arm64 and x86_64), Developer ID signature with the hardened runtime, notarized by `notarytool` and stapled (`scripts/build-release.sh`), so Gatekeeper opens a download | Unsigned: SmartScreen warns on first run. Authenticode would need a certificate | Unsigned packages, as usual; nothing checks |
| Install location for local release builds | `~/Applications/Coupler.app` via `ditto` (`scripts/build-release.sh`) | n/a (installer) | n/a (package) |
| Full screen | Native full screen (its own Space) through Tauri's `set_fullscreen`; tao makes the fixed-size window resizable for the switch and restores it after. Key Ctrl+Cmd+F | Borderless full screen through the same call. Key Ctrl+Alt+F (`lib/keys.ts`) | Same call; depends on the window manager. Key Ctrl+Alt+F |
| Immersive's voice | WebKit's `speechSynthesis` (the system voices, AVSpeechSynthesizer); `lib/voice.ts`. Planned: `AVSpeechSynthesizer` from Rust, its audio through the mixer | WebView2's `speechSynthesis` (SAPI/OneCore voices); natively SAPI or OneCore | WebKitGTK may have no `speechSynthesis`: Speech Dispatcher (check its client library's license: LGPL rules it out of the binary) or bundled Flite (BSD-style). Not eSpeak NG (GPL-3) |
| Characters' voices in Flite (`synth.rs`) | Pure Rust (`flite-rs`), played through Web Audio: nothing macOS-specific | Same | Same; and Flite stands in for a missing `speechSynthesis` there |
| Characters' voices in Pocket TTS (`synth.rs`, `vendor/pocket-tts`) | Candle on the CPU, pure Rust, the files in `Coupler.app/Contents/Resources/pocket-tts` (`resource_dir`) | Same; the files beside the executable in the installer's resources. CPU speed varies more: check a line still comes in time | Same; check the package puts the resources where `resource_dir` looks |
| Characters' voices by gender | `lib/voice.ts` `GENDERS`: macOS's voice names (Samantha, Daniel, …) marked feminine or masculine, and its novelty voices (Zarvox, …) left out of Automatic; `speechSynthesis` says nothing of a voice's gender | Windows' voice names (Zira, David, …) need their own entries | Whatever engine speaks: its voices' names, or its own variants (Flite's voices are named by speaker) |
| The game's talk in bundled voices (planned) | Pocket TTS in Rust (Candle, Metal) | Same, CPU | Same, CPU |
| The voice cache's free-space check (`voicecache.rs` `free_space`; Storage's free space too, `storage.rs`) | `statvfs` (libc) on the app-data volume; `f_bavail` × `f_frsize` | `GetDiskFreeSpaceExW` (not built: until then the cache keeps to its 1 GB cap alone) | `statvfs`, as on macOS (built, untested) |
| Opening the release page and About's Discord and Reddit (`update.rs` `open_page`) | `open` with the fixed URL | `rundll32 url.dll,FileProtocolHandler` | `xdg-open` (untested) |
| The update check's TLS (`update.rs`, reqwest `native-tls`) | Security framework | SChannel | The system's OpenSSL, linked (`libssl-dev` in `linux.yml`), never bundled |
| Keyboard shortcuts (`lib/keys.ts`) | Cmd+Shift+letter, Ctrl+Cmd+key, Option+arrow, Cmd+Period; written that way | Cmd is Ctrl, Ctrl+Cmd is Ctrl+Alt, Option is Alt, Return is Enter, matched and written (`keys.keys`) alike: the Windows key belongs to the system. Ctrl+Alt is AltGr on some layouts | Same as Windows; the Super key belongs to the desktop |
| Is a screen reader running (`screen_reader.rs`) | VoiceOver's process, `pgrep -x VoiceOver`, every 5 seconds | `SystemParametersInfoW(SPI_GETSCREENREADER)`, set by NVDA, JAWS and Narrator (built, untested) | GNOME's `screen-reader-enabled`, or Orca's process (built, untested) |
| Bundle ID (`tauri.conf.json`, rule 4) | `com.scorelessmusic.coupler`; 0.28.0 and before were `com.ansiapps.coupler`, whose `~/Library/Application Support` and `~/Library/WebKit` folders `moved.rs` renames at launch | `com.ansiapps.coupler` (`tauri.windows.conf.json`), unchanged | `com.ansiapps.coupler` (`tauri.linux.conf.json`), unchanged |
| Quit Coupler (gear menu) | `AppHandle::exit`; Cmd+Q from the app menu too | Same call | Same call |
| Fixed window size | `resizable: false` greys out the green button; a window taller than the screen is shrunk by the system, and the stage scales down to fit | Check 1366×768 with the taskbar: the window is taller than the work area | Check tiling window managers, which ignore fixed sizes (the stage scales to whatever it's given) |

## The iPhone (begun 2026-10-19)

The phone screen (`lib/formFactor.ts`, `phoneLayout` in `App.tsx`,
`src/phone.css`): upright only, iPhone only, played by touch. The same
code as the Mac's; what differs:

| Feature | iPhone | Note |
|---|---|---|
| The screen | 48 cells across, as many rows as show above the keyboard (`lib/stage.ts` `phoneFit`), scaled to the width; the bar, the touch pad and the command line 3 rows (48 px) each | The desktop's 1280 by 720 is untouched |
| Minimum iOS | iOS 16 (the iPhone 8 and later): Candle's `gemm-f16` needs the A11's half-precision instructions, turned on in `.cargo/config.toml` | |
| Orientation and devices | Portrait only, iPhone only (`project.yml`: `UISupportedInterfaceOrientations`, `TARGETED_DEVICE_FAMILY` 1); an iPad runs it as an iPhone app | An iPad layout would be its own profile |
| The keyboard | `visualViewport` sizes the frame to what shows, so the command line sits on the keyboard; fields at 16 px so iOS doesn't zoom | |
| Notch and home indicator | `viewport-fit=cover` and `env(safe-area-inset-*)` on the frame | |
| The first-run warning | None: App Review turns down an app calling itself unfinished (`mod first_run` is desktop only) | |
| Is a screen reader running (`screen_reader.rs`) | `UIAccessibilityIsVoiceOverRunning` (an iPhone app can't start `pgrep`) | |
| About's Discord and Reddit | Left out: `update.rs` opens pages by a desktop's own opener | Needs the iOS opener (`tauri-plugin-opener` or `UIApplication.open`) |
| App Store privacy manifest | `gen/apple/coupler_iOS/PrivacyInfo.xcprivacy`: no tracking, nothing collected, the reasons for file times, boot time, disk space, user defaults | Keep it in step with new system APIs |
| Sound with the ring switch on silent | `navigator.audioSession.type = "playback"` (iOS 16.4 and later) | Earlier iOS: the switch mutes it |
| Full screen, Quit, Keyboard, backups, the update check, arranging | Left out of the gear menu | An App Store app is never quit, updated or side-loaded from inside |
| Signing and release | Xcode's Apple Distribution and the App Store (`npx tauri ios build`, team in `tauri.conf.json` `bundle.iOS`) | Not yet: running in the background, the screen reader's check (`screen_reader.rs`) |

## The iPad (begun 2026-10-23)

The tablet screen (`lib/formFactor.ts` `TABLET_SCREEN`, `tabletLayout`
in `App.tsx`, `src/tablet.css`): upright only, played by touch, today in
the web build (the iOS app is iPhone only). An iPhone in the web build
gets it too (decided 2026-10-24): the phone screen is the iPhone app's.
On a 390-point iPhone only the smallest text leaves the desk its 40
columns (`tabletFit`'s `largest`). What differs:

| Feature | iPad | Note |
|---|---|---|
| The screen | Two stages (`lib/stage.ts` `tabletFit`): Workshop's desk above, in whole cells of the player's text size (`lib/tablet.ts`, 8 by 16 to 16 by 32 points); the lower part 96 cells by 23 rows, scaled to the width | The ways to play's chooser is left out: the desk is Workshop's |
| The keyboard | Coupler's own (`components/TabletKeyboard.tsx`, `lib/softKeys.ts`): every text field gets `inputmode="none"` (a MutationObserver), so the system's never shows; keys are synthetic keydowns, then the edit done by hand | A hardware keyboard still types as usual |
| Zoom | `maximum-scale=1, user-scalable=no` set at launch, Safari's `gesturestart` and two-finger `touchmove` stopped, `touch-action: pan-x pan-y` | iOS ignores the viewport's `user-scalable` for a pinch, hence the gesture events |
| Orientation | `screen.orientation.lock("portrait")` where a browser allows it (none on iOS); otherwise a notice covers the screen held sideways | An iPad app would set `UISupportedInterfaceOrientations~ipad` |
| Full screen | Left out | |
| Installing | The web build's `manifest.webmanifest` and `apple-touch-icon` (`web/`, added by `vite.config.ts` `installable`): Share → Add to Home Screen, opening without Safari's bars. No service worker | Chrome and Edge's Install and Safari's Add to Dock on a computer, from the same manifest |

The connection itself (`std::net::TcpStream`, `flate2`) is the same on
every platform. Planned, add a row when each lands: saved passwords
(macOS Keychain; Windows Credential Manager; Linux Secret Service via
libsecret/D-Bus), sound playback for MSP/Client.Media, notifications for
tells.
