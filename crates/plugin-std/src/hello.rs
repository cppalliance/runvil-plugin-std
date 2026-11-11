//! The `hello` tool: a temporary tool that takes no arguments and returns
//! a fixed greeting. It exists to prove that a prompt can call a tool of a
//! library Plugin, and it goes away when the file tools arrive.

use runvil_plugin_sdk::{ToolInfo, ToolOutput};

/// The tool's name within the Pack, so its Lua id is `std/hello`.
pub(crate) const NAME: &str = "hello";

/// What a call returns.
pub(crate) const GREETING: &str = "Hello from the std Plugin.";

/// The tool as the Pack lists it. A model calls it `Hello`.
pub(crate) fn info() -> ToolInfo {
    ToolInfo::new(
        NAME,
        "Returns a fixed greeting from the std Plugin. Takes no arguments.",
        r#"{"type":"object","properties":{}}"#,
    )
    .wire_name("Hello")
}

/// Performs one call: the greeting, which the Plugin vouches for.
pub(crate) fn call() -> ToolOutput {
    ToolOutput::trusted(GREETING)
}

#[cfg(test)]
mod tests {
    use runvil_plugin_sdk::{ToolOutput, testing};

    use super::{GREETING, info};
    use crate::Std;

    #[test]
    fn the_tool_is_listed_as_hello_with_the_wire_name_a_model_calls() {
        let loaded = testing::load::<Std>().unwrap();
        let pack = loaded.new_pack("runvil/std").unwrap();
        assert_eq!(pack.tools(), [info()]);
        assert_eq!(pack.tools()[0].wire_name.as_deref(), Some("Hello"));
    }

    #[test]
    fn a_call_returns_the_greeting_as_trusted_output() {
        let loaded = testing::load::<Std>().unwrap();
        let pack = loaded.new_pack("runvil/std").unwrap();
        assert_eq!(
            pack.call("hello", "{}"),
            Ok(Ok(ToolOutput::trusted(GREETING)))
        );
    }
}
