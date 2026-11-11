//! `plugin-std` - the `std` Plugin. At contract 0.2 it loads, describes
//! itself as `std` at its crate version, and unloads; its Pack comes once
//! the contract can insert one.
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

use runvil_plugin_sdk::{Plugin, Refusal};

/// The Plugin instance the Host holds while the Plugin is loaded. It has
/// no state: nothing is shared between runs.
#[derive(Debug)]
pub struct Std;

impl Plugin for Std {
    const NAME: &'static str = "std";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn load() -> Result<Std, Refusal> {
        Ok(Std)
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
}
