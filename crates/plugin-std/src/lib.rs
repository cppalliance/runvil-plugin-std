//! `plugin-std` - the `std` Plugin. It inserts one Pack, `runvil/std`,
//! whose Pack id is `std`. In this build the Pack offers one temporary
//! tool, `std/hello`, which proves the path from a prompt to a library
//! Plugin before the file tools move here.
//!
//! `cargo build -p plugin-std --release` writes `plugin_std.dll` on
//! Windows, `libplugin_std.so` on Linux, and `libplugin_std.dylib` on
//! macOS to `target/release`; that one file is what goes in a Host's
//! Plugin directory.
//!
//! ## Invariants
//!
//! - Uses `runvil-plugin-sdk` alone and writes no `unsafe`.
//! - The `dll` feature exports `runvil_plugin`. A Host that links the
//!   crate in turns the feature off and hands
//!   `runvil_plugin_sdk::entry::<Std>` to the Harness.
//! - The source of each tool sits in its own module, named after the last
//!   segment of the tool's Lua id.

mod hello;
mod pack;

use runvil_plugin_sdk::{Pack, PackInfo, Plugin, Refusal};

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
        Ok(Box::new(pack::StdPack))
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
