# runvil-plugin-std

The `std` Plugin for [runvil](https://github.com/cppalliance/runvil), built as a dynamic library that a Host loads from its Plugin directory. It speaks the C contract in [runvil-plugin-api](https://github.com/cppalliance/runvil-plugin-api) through `runvil-plugin-sdk`, and depends on nothing else from runvil.

The Plugin inserts one Pack, `runvil/std`, which gives a run the standard file tools a coding agent needs. A prompt declares it with `uses: [std]`, a script calls a tool by its Lua id, and a model calls it by its wire name. Each wire name is Cursor's name for the same tool, except `ListDir`, which is this Pack's own.

| Lua id | Name the model calls | In this build |
|---|---|---|
| `std/read` | `Read` | Returns a window of a text file's lines, each with its line number |
| `std/dir` | `ListDir` | Returns a bullet tree of a directory, listed breadth first |
| `std/grep` | `Grep` | Searches the files under a path with a Rust regular expression |
| `std/write` | `Write` | A stub: answers that the tool is not implemented |
| `std/replace` | `StrReplace` | A stub |
| `std/rm` | `Delete` | A stub |
| `std/glob` | `Glob` | A stub |

- Each tool's wire name, description, and parameter schema is a JSON file under `crates/plugin-std/schemas`, and `EDITS.md` beside them lists how each differs from Cursor's definition.
- Every file a tool reads goes through the Host, so the run's policy applies to each read. A read the policy denies fails the call with the policy's own reason. The one exception is an entry below the path a Grep call names, which Grep skips and counts.
- File text and file names come back as untrusted output. The Host marks them before a model reads them.
- A search ends its step every 32 files or directories, or every 2 MiB read, so the Host can cancel it between two steps.

## Building

```text
cargo build -p plugin-std --release
```

This writes `plugin_std.dll` (Windows), `libplugin_std.so` (Linux), or `libplugin_std.dylib` (macOS) to `target/release`. That one file goes in the Host's Plugin directory.

A Host can also link the crate in. It turns the `dll` feature off, so the crate exports no symbol, and hands `runvil_plugin_sdk::entry::<plugin_std::Std>` to the Harness.

## Working on this repository

| Task | Command |
|---|---|
| Test, including the structural checks | `cargo test --locked --workspace --all-features` |
| Lint, with `CARGO_BUILD_WARNINGS=deny` | `cargo clippy --locked --workspace --all-targets --all-features` |
| Format | `cargo fmt --all --check` |
| Docs, with `RUSTDOCFLAGS="-D warnings"` | `cargo doc --locked --workspace --no-deps --all-features --document-private-items` |
| The library Plugin | `cargo build --locked -p plugin-std --release` |

CI runs these five. `AGENTS.md` holds the rules the code follows. Sizes are limited on functions and types, never on files: `clippy.toml` sets the function limits, and `crates/build-checks` checks the type limits as a test.

## Platform notes

- Windows: the Harness resolves a library Plugin's own DLL dependencies only from the Plugin's directory and System32. This repository links with the static CRT (`.cargo/config.toml`), so the Plugin does not need the MSVC runtime DLLs.
- Keep Cargo's default `panic = "unwind"`. With `panic = "abort"`, a panic in the Plugin aborts the Host's process instead of becoming a refusal or a failed tool call.

## License

Boost Software License 1.0; see `LICENSE`.
