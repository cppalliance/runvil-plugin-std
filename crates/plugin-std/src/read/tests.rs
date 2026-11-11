//! Unit tests for the `Read` tool. This file holds the helpers the topic
//! files share: a tree of files held in memory, and the two ways to drive
//! Read over it, through its arguments and through a lowered byte cap.

mod arguments;
mod failures;
mod paging;
mod render;
mod window;

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{ToolError, ToolOutput};
use serde_json::Value;

use super::args::Request;
use crate::source::Source;
use crate::spec::{self, Definition};

/// A tree with `bytes` at `path`.
fn seeded(path: &str, bytes: &[u8]) -> Files {
    Files::new().file(path, bytes)
}

/// The `read` definition the Pack builds the tool from.
fn definition() -> &'static Definition {
    spec::all()
        .expect("every embedded definition parses")
        .iter()
        .find(|definition| definition.segment == "read")
        .expect("the read definition exists")
}

/// Calls Read over `files` with `args` as the Pack does: the schema check
/// first, then the tool.
fn read(files: &impl Source, args: &Value) -> Result<ToolOutput, ToolError> {
    let definition = definition();
    crate::check::check(&definition.wire_name, &definition.parameters, args)?;
    super::call(files, definition, args)
}

/// Reads `path` with the window `offset` and `limit` under the byte cap
/// `cap`.
fn read_capped(
    files: &Files,
    path: &str,
    offset: Option<i64>,
    limit: Option<usize>,
    cap: usize,
) -> Result<ToolOutput, ToolError> {
    let request = Request {
        path,
        offset,
        limit,
    };
    super::read_file(files, "Read", &request, cap)
}

/// The text of a successful read, which must be untrusted output.
fn text_of(result: Result<ToolOutput, ToolError>) -> String {
    let output = result.expect("the read succeeds");
    assert!(!output.is_trusted(), "{}", output.text());
    output.text().to_owned()
}

/// The message of a failed read.
fn failure_of(result: Result<ToolOutput, ToolError>) -> String {
    match result {
        Ok(output) => panic!("the read fails, but it answered {:?}", output.text()),
        Err(error) => error.to_string(),
    }
}

/// A file of `count` lines, `line 1` to `line {count}`, with or without a
/// final newline.
fn numbered_file(count: usize, final_newline: bool) -> String {
    let mut text = (1..=count)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    if final_newline && count > 0 {
        text.push('\n');
    }
    text
}
