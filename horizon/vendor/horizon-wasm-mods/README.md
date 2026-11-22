# Horizon's wasm mod runtime (vendored)

These crates come from [Horizon](https://github.com/horizon-rust/horizon) at commit
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, the SDK commit Horizon's current `.hmod` packages
(api `"0.2"` with a `[wasm]` table) are made for. They are licensed under the GNU Lesser General
Public License, version 3 or later: see [LICENSE](LICENSE), with the GNU GPL it builds on in [COPYING.GPL-3](COPYING.GPL-3). Pojarush links them into the game
to run those packages (`crates/pojarush-wasm-mods`).

What Pojarush changed:

- Only the crates the runtime needs are kept; tests, benches, examples and binaries are left out.
  `horizon-core` is renamed to the package `horizon-up-core` (its dependents still call it
  `horizon_core`) so it can sit beside Pojarush's own `horizon-core`.
- `horizon-mod-api`, `horizon-mod-vocab`, `horizon-mod-points`, `horizon-engine-slots` and
  `horizon-mod-wasm` generate code from Horizon's whole engine source at build time. That code
  was generated once at the commit above and is kept, gzipped, in each crate's `pregenerated/`;
  their `build.rs` unpacks it instead of running the generator.
- `horizon-guest-stack`: Android support (a thread's reserved stack read with
  `pthread_getattr_np`; the app's main thread never hosts guests).
- `horizon-mod-wasm`: `load_cached`, which compiles in the game's own process (an Android app
  can't start a compiler executable) and keeps the compiled modules in a cache folder.
- `horizon-mod-vocab`: `Corners::parts` and `Corners::from_parts`.

`horizon-wasm-std.wasm` is Horizon's standard library for guests, built from these sources with
`cargo horizon-mod wasm-std` and the toolchain the packages name (Rust 1.99.1,
`58cf8f9c35b355199727627d07ae34a4e2846d67`).
