# Coupler's copy of wavpack-sys 0.4.0

`wavpack-sys` 0.4.0 from crates.io (BSD-3-Clause, gitlab.com/Toru3/wavpack-rs),
with WavPack's C source (BSD-3-Clause, `WavPack/COPYING`), used through
`[patch.crates-io]` in `src-tauri/Cargo.toml`. One change, in `build.rs`:
WavPack is linked `static=wavpack` (the crate left the kind out), so
it's packed into the static library the iPhone's Xcode project links,
as `opusic-sys` does for Opus. The Mac, Windows and Linux builds link
the same `.a` either way. Drop this copy once the crate links statically.

---

# wavpack-rs

Rust bindings for [WavPack](https://www.wavpack.com/). 

The included WavPack version is [5.6.0](https://www.wavpack.com/changelog.txt).

## Current Status 

Reading and writing support is implemented and appears to work correctly,
although is under-tested.

Not all APIs are exposed, but please feel encouraged to open PRs for anything 
that you need that's missing. APIs that have been exposed are thin wrappers for
calls to the underlying WavPack functions, for usage information see the 
[WavPack API documentation](https://www.wavpack.com/WavPack5LibraryDoc.pdf).

## Dependencies

* [CMake](https://cmake.org) is used to build WavPack.  
* A C++ compiler.

## Acknowledgments

Many thanks to [Toru3](https://toru3.gitlab.io) who started this project.
