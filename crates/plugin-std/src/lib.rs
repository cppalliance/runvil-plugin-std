//! `plugin-std` - the `std` Plugin. It inserts one Pack, `runvil/std`,
//! which gives a run the standard file tools a coding agent needs: read,
//! write, string replace, delete, glob, grep, and list directory.
//!
//! The Pack's id is `std`, and a run holds all of its tools unless it
//! declares another vendor's `std`:
//!
//! | Lua id | Name the model calls |
//! |---|---|
//! | `std/read` | `Read` |
//! | `std/write` | `Write` |
//! | `std/replace` | `StrReplace` |
//! | `std/rm` | `Delete` |
//! | `std/glob` | `Glob` |
//! | `std/grep` | `Grep` |
//! | `std/dir` | `ListDir` |
//!
//! A script calls a tool by its Lua id, `tools.call("std/read", { path =
//! "/notes.md" })`, and offers it to the model with `tools.offer("std")`
//! or `tools.offer("std/read")`. The model then calls it by its wire name,
//! `Read`, which is Cursor's name for the same tool.
//!
//! The Pack has no prelude, so declaring it, as in `uses: [std]`, only
//! makes it required. It takes no configuration.
//!
//! `cargo build -p plugin-std --release` writes `plugin_std.dll` on
//! Windows, `libplugin_std.so` on Linux, and `libplugin_std.dylib` on
//! macOS to `target/release`; that one file is what goes in a Host's
//! Plugin directory.
//!
//! # The definitions
//!
//! Each tool's description and parameter schema live in a JSON file under
//! `schemas/`, embedded at build time. They are Cursor's own
//! definitions, so a model that knows Cursor's tools already knows these,
//! with the lines that describe behavior this Pack does not have removed.
//! `schemas/EDITS.md` lists every such edit. ListDir's definition is this
//! crate's own, because Cursor's counterpart is deprecated, and the same
//! file says so.
//!
//! # This build
//!
//! Read is real. It returns a window of a text file's lines, each with a
//! six-wide line number, and it refuses a directory, a file over 16 MiB,
//! and binary data. A read that stops early ends with a line that names
//! the offset to continue from. File text comes back as untrusted output.
//!
//! ListDir is real. It returns a bullet tree of a directory, listed breadth
//! first, with a trailing `/` on each directory, files and directories in
//! one byte-ordered list, and every entry named `.git` skipped. A link is
//! listed and not entered, and an `ignore_globs` pattern leaves out what
//! it matches at any depth. The tree holds at most 15,000 bytes of lines,
//! and a listing that reaches that says so. Names come back as untrusted
//! output.
//!
//! Grep is real. It searches the files under a path with a Rust regular
//! expression, in path order, and prints each matching line as `N:text`
//! with optional context lines as `N-text`, where `N` is the line number
//! Read shows. Output stops at 15,000 bytes, a call with an `offset`
//! pages through the same result, and a binary file, a link, an over-size
//! file, and an entry that cannot be read are skipped and counted, never
//! a failure. A search ends its step now and then, so a cancel or a stop
//! can drop it between two steps. File text comes back as untrusted
//! output.
//!
//! Write, StrReplace, Delete, and Glob are still stubs: a call with
//! arguments that match the schema answers [`NOT_IMPLEMENTED`]. For every
//! tool, a call whose arguments do not match fails with an
//! invalid-arguments [`ToolError`](runvil_plugin_sdk::ToolError) naming
//! the parameter.
//!
//! # Where the file access goes
//!
//! Every filesystem operation a tool performs goes through its
//! [`ToolCall`](runvil_plugin_sdk::ToolCall), never the disk directly.
//! The Host answers each one under the run's mode and policy, so a read
//! the policy denies fails without the Pack deciding anything. The tools
//! only report the resulting
//! [`FileError`](runvil_plugin_sdk::FileError) as a tool error the model
//! can read.
//!
//! Shell is deliberately absent. An in-process shell over the same
//! filesystem is a separate Pack, so a run that wants one declares it.
//!
//! ## Invariants
//!
//! - Depends on `runvil-plugin-sdk` and outside libraries alone, and
//!   writes no `unsafe`.
//! - The `dll` feature exports `runvil_plugin`. A Host that links the
//!   crate in turns the feature off and hands
//!   `runvil_plugin_sdk::entry::<Std>` to the Harness.
//! - The crate names no async runtime and no filesystem of its own. Every
//!   operation reaches files through the call's `ToolCall`.
//! - The source of each tool sits in its own module, named after the last
//!   segment of the tool's Lua id.
//! - A tool's description and parameter schema come from its JSON file
//!   unchanged, except for the edits `schemas/EDITS.md` lists. A test per
//!   tool holds the wire name in place.
//! - A call whose arguments the schema refuses is an
//!   [`InvalidArguments`](runvil_plugin_sdk::ToolErrorKind::InvalidArguments)
//!   failure that names the parameter. No other outcome reports the
//!   arguments, so a model can tell a bad call from a missing
//!   implementation.
//! - A key the schema does not declare is refused by name, with the
//!   parameters listed, by the three real tools, Read, ListDir, and
//!   Grep. The stubs let it through. Grep alone accepts `-n` when it is
//!   `true`, because Cursor's definition once declared it.

mod check;
mod dir;
mod grep;
mod matcher;
mod order;
mod pack;
mod read;
mod refusal;
mod source;
mod spec;
mod text;

use runvil_plugin_sdk::{Pack, PackInfo, Plugin, Refusal};

pub use crate::pack::NOT_IMPLEMENTED;

/// The Plugin instance the Host holds while the Plugin is loaded. It has
/// no state: nothing is shared between runs.
#[derive(Debug)]
pub struct Std;

impl Plugin for Std {
    const NAME: &'static str = "std";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const PACKS: &'static [PackInfo] = &[pack::INFO];

    fn load() -> Result<Std, Refusal> {
        Ok(Std)
    }

    fn new_pack(&self, _pack: &str) -> Result<Box<dyn Pack>, Refusal> {
        Ok(Box::new(pack::StdPack::new()?))
    }
}

#[cfg(feature = "dll")]
runvil_plugin_sdk::export_plugin!(Std);

#[cfg(test)]
mod tests {
    use runvil_plugin_sdk::sys::RV_OK;
    use runvil_plugin_sdk::testing;

    use super::Std;

    #[test]
    fn the_plugin_loads_as_std_at_its_version_and_unloads() {
        let loaded = testing::load::<Std>().unwrap();
        assert_eq!(loaded.name(), "std");
        assert_eq!(loaded.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(loaded.unload(), RV_OK);
    }

    #[test]
    fn the_plugin_inserts_the_one_pack_runvil_std() {
        let loaded = testing::load::<Std>().unwrap();
        let packs: Vec<(&str, &str)> = loaded
            .packs()
            .iter()
            .map(|pack| (pack.name.as_str(), pack.description.as_str()))
            .collect();
        assert_eq!(
            packs,
            [("runvil/std", "Read, write, search, and delete files")]
        );
    }
}
