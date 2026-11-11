//! The `std` Pack: its [`INFO`], and [`StdPack`], one run's instance,
//! which lists the Pack's tools and hands each call to its tool's module.

use runvil_plugin_sdk::{
    BoxFuture, Pack, PackInfo, ToolCall, ToolError, ToolErrorKind, ToolInfo, ToolOutput,
};

use crate::hello;

/// The Pack's name and description. Its name is `runvil/std`, so its
/// Pack id is `std`, and a prompt reaches its tools as `std/<name>`.
pub(crate) const INFO: PackInfo =
    PackInfo::new("runvil/std").description("Read, write, search, and delete files");

/// One run's instance of the Pack. It has no state: each call works only
/// on what the call lends it.
pub(crate) struct StdPack;

impl Pack for StdPack {
    fn tools(&self) -> Vec<ToolInfo> {
        vec![hello::info()]
    }

    fn call<'a>(&'a self, call: ToolCall<'a>) -> BoxFuture<'a, Result<ToolOutput, ToolError>> {
        Box::pin(async move {
            match call.tool() {
                hello::NAME => Ok(hello::call()),
                name => Err(ToolError::new(
                    ToolErrorKind::Other,
                    format!("std has no tool named {name}"),
                )),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use runvil_plugin_sdk::testing;

    use crate::Std;

    #[test]
    fn an_instance_offers_each_tool_once() {
        let loaded = testing::load::<Std>().unwrap();
        let pack = loaded.new_pack("runvil/std").unwrap();
        let names: Vec<&str> = pack.tools().iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names, ["hello"]);
    }
}
