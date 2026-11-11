//! The `std` Pack: its [`INFO`], [`StdPack`], one run's instance, which
//! lists the Pack's tools and hands each call to its tool's module, and
//! the stub that every tool but Read, ListDir, and Grep still answers
//! with in this build.

use std::sync::Arc;

use runvil_plugin_sdk::{
    BoxFuture, Pack, PackInfo, ToolCall, ToolError, ToolErrorKind, ToolInfo, ToolOutput,
};
use serde_json::Value;

use crate::check::make_invalid_arguments_error;
use crate::source::Source;
use crate::spec::Definition;

/// What a call to a tool that is still a stub answers in this build, when
/// its arguments match the schema. Read, ListDir, and Grep are real and
/// never answer this.
///
/// The text says the implementation is missing rather than that the call
/// was wrong, because a model that reads it should stop calling the tool.
/// It also says a repeat call is safe, so a model that retries anyway
/// learns the outcome is the same rather than assuming the call was
/// malformed. `<name>` is the tool's wire name.
pub const NOT_IMPLEMENTED: &str = "The <name> tool is not implemented in this build. The \
    implementation is missing and nothing in this session will change that. Calling <name> again \
    is safe, but it will return this same result every time, whatever the arguments. Carry on \
    without it.";

/// The Pack's name and description. Its name is `runvil/std`, so its
/// Pack id is `std`, and a prompt reaches its tools as `std/<name>`.
pub(crate) const INFO: PackInfo =
    PackInfo::new("runvil/std").description("Read, write, search, and delete files");

/// One run's instance of the Pack: the definitions the Plugin parsed when
/// it loaded, which every run shares, and the tool list made from them.
/// It keeps nothing of a call, so each call works only on what the call
/// lends it.
#[derive(Debug)]
pub(crate) struct StdPack {
    definitions: Arc<[Definition]>,
    tools: Vec<ToolInfo>,
}

impl StdPack {
    /// Makes the instance, naming each tool by its definition's segment.
    pub(crate) fn new(definitions: Arc<[Definition]>) -> StdPack {
        let tools = definitions
            .iter()
            .map(|definition| {
                ToolInfo::new(
                    definition.segment,
                    definition.description.clone(),
                    definition.parameters.to_string(),
                )
                .wire_name(definition.wire_name.clone())
            })
            .collect();
        StdPack { definitions, tools }
    }
}

/// The stub text for the tool the model calls `wire_name`.
fn stub(wire_name: &str) -> String {
    NOT_IMPLEMENTED.replace("<name>", wire_name)
}

/// The call's arguments, parsed from the JSON text the Host sent for the
/// tool the model calls `wire_name`.
fn parse(wire_name: &str, text: &str) -> Result<Value, ToolError> {
    serde_json::from_str(text).map_err(|error| {
        make_invalid_arguments_error(wire_name, format!("the arguments are not JSON: {error}"))
    })
}

/// Runs one call of the tool of `definitions` named `name` with
/// `arguments`, the JSON text of its arguments, over the files `source`
/// reaches: the schema check first, then the tool.
async fn run(
    definitions: &[Definition],
    name: &str,
    arguments: &str,
    source: &impl Source,
) -> Result<ToolOutput, ToolError> {
    let found = definitions
        .iter()
        .find(|definition| definition.segment == name);
    let Some(definition) = found else {
        return Err(ToolError::new(
            ToolErrorKind::Other,
            format!("std has no tool named {name}"),
        ));
    };
    let args = parse(&definition.wire_name, arguments)?;
    crate::check::check(&definition.wire_name, &definition.parameters, &args)?;
    match definition.segment {
        "read" => crate::read::call(source, definition, &args),
        "dir" => crate::dir::call(source, definition, &args),
        "grep" => crate::grep::call(source, definition, &args).await,
        _ => Ok(ToolOutput::trusted(stub(&definition.wire_name))),
    }
}

impl Pack for StdPack {
    fn tools(&self) -> Vec<ToolInfo> {
        self.tools.clone()
    }

    fn call<'a>(&'a self, call: ToolCall<'a>) -> BoxFuture<'a, Result<ToolOutput, ToolError>> {
        Box::pin(async move { run(&self.definitions, call.tool(), call.arguments(), &call).await })
    }
}

#[cfg(test)]
#[path = "pack-tests.rs"]
mod tests;
