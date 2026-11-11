# runvil-plugin-std

The `std` Plugin for [runvil](https://github.com/cppalliance/runvil), built as a dynamic library that a Host loads from its Plugin directory. It speaks the C contract in [runvil-plugin-api](https://github.com/cppalliance/runvil-plugin-api) through `runvil-plugin-sdk`, and depends on nothing else from runvil.

At contract 0.2 a library Plugin can only load, describe itself, and unload, so that is all this Plugin does. Its Pack, `runvil/std`, comes once the contract can insert one.

## Building

```text
cargo build -p plugin-std --release
```

This writes `plugin_std.dll` (Windows), `libplugin_std.so` (Linux), or `libplugin_std.dylib` (macOS) to `target/release`. That one file goes in the Host's Plugin directory.

A Host can also link the crate in. It turns the `dll` feature off, so the crate exports no symbol, and hands `runvil_plugin_sdk::entry::<plugin_std::Std>` to the Harness.

## Working on this repository

| Task | Command |
|---|---|
| Test, including the structural checks | `cargo test --workspace --all-features` |
| Lint | `cargo clippy --workspace --all-targets --all-features` |
| Format | `cargo fmt --all --check` |
| Docs | `cargo doc --workspace --no-deps --all-features` |

`AGENTS.md` holds the rules the code follows. Sizes are limited on functions and types, never on files: `clippy.toml` sets the function limits, and `crates/build-checks` checks the type limits as a test.

## Platform notes

- Windows: the Harness resolves a library Plugin's own DLL dependencies only from the Plugin's directory and System32. This repository links with the static CRT (`.cargo/config.toml`), so the Plugin does not need the MSVC runtime DLLs.
- Keep Cargo's default `panic = "unwind"`. With `panic = "abort"`, a panic in the Plugin aborts the Host's process instead of becoming a refusal.

## License

Boost Software License 1.0; see `LICENSE`.
