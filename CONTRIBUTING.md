# Contributing to Coupler

Thank you for helping. Coupler is small and opinionated: please open an
issue to talk about anything bigger than a fix before you build it.

## The rules a change has to keep

The full list is in `CLAUDE.md`. The ones most often missed:

1. **CoffeeMUD only.** Coupler connects to coffeemud.net's games and
   nowhere else. No setting, command or file takes another host or
   port, and no new network code without an issue first. The one
   exception is the update check, which asks GitHub for Coupler's
   latest release only when the player turns it on or asks.
2. **No server, no telemetry, no accounts.** Nothing about the player
   leaves their computer except what they send the game.
3. **Credentials stay local.** A password is never echoed, logged or
   kept in history.
4. **GitHub Actions only where it's free and unlimited**: standard
   GitHub-hosted runners (`ubuntu-*`, `windows-*`, `macos-*`) in this
   public repository. No larger or self-hosted runners, no paid
   actions or services, no raised cache limit.
5. **No GPL, AGPL or LGPL dependencies**, so the license stays a
   choice. When `Cargo.lock` or `package-lock.json` changes, run
   `scripts/license-scan.py` and `scripts/third-party-licenses.py` and
   commit what they write.
6. **The protocol lives in Rust** (`telnet.rs`, `ansi.rs`), pure and
   unit-tested. A bug about garbled output starts as a byte-level
   regression test there.
7. **Accessible first: an immersive game for every player**, sighted
   or not, heard as well as read. Every control and feature answers six
   questions (`docs/accessibility.md`): it works from the keyboard
   alone; it's named and announced for VoiceOver; nothing is said only
   by a picture, color or position; it doesn't speak too much; it's
   readable at low vision; focus goes somewhere sensible. Test new UI
   with VoiceOver on.
8. **A fixed screen on the ANSIapps grid.** The window is 1280 by 720.
   Lay out in stage pixels on the 8 by 16 grid (`docs/ansiapps-theme.md`)
   and run the dev gear menu's Check the Grid on what you changed.
9. **Pictures are Coupler's painter's first** (`painter.rs`,
   `portrait.rs`). Other sources are the player's choice, never the
   default.

## Before you open a pull request

- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets`
- `npx tsc --noEmit`
- If you touched `src-web/` or a module it shares: `cargo test` and
  `cargo clippy --all-targets` in `src-web/`, and `npm run build:web`.
- Add a bullet to `CHANGELOG.md` under Unreleased.
- **No game logs or captures** in a commit, an issue or a pull request
  unless you've removed other players' names and chat.
- **Don't test against coffeemud.net with scripts.** It's a shared
  server with real players. For protocol work, run a local CoffeeMUD
  (Java, Apache-2.0) and point a local build at it; never commit that
  change.

## Neumetik

`src-tauri/vendor/neumetik/` is a copy of ansiapps' synthesizer, kept
in step with its own private repository. Coupler doesn't take changes
to that folder: open an issue describing the problem instead.

## Signing off your commits

Coupler uses the [Developer Certificate of Origin](https://developercertificate.org/).
Add a `Signed-off-by` line to each commit (`git commit -s`) to certify
that you wrote the change or have the right to submit it under
Coupler's license, Apache-2.0.

## Conduct

Be kind and assume good faith. Harassment, insults and personal attacks
aren't welcome in Coupler's issues, pull requests or discussions, and
the maintainer may remove them and block whoever posts them.
