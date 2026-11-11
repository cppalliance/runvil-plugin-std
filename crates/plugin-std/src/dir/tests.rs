//! Unit tests for the `ListDir` tool. This file holds the helpers the
//! topic files share: a tree of files held in memory, and the two ways to
//! drive ListDir over it, through its arguments and through a lowered
//! byte cap.

mod arguments;
mod cap;
mod ignore;
mod tree;

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{ToolError, ToolOutput};
use serde_json::Value;

use super::args::Request;
use crate::matcher::Matcher;
use crate::source::Source;
use crate::spec::{self, Definition};

/// A tree with one small file at each of `paths`, and every directory on
/// the way to it.
fn seeded(paths: &[&str]) -> Files {
    paths
        .iter()
        .fold(Files::new(), |files, path| files.file(path, b"x"))
}

/// The `dir` definition the Pack builds the tool from.
fn definition() -> &'static Definition {
    spec::all()
        .expect("every embedded definition parses")
        .iter()
        .find(|definition| definition.segment == "dir")
        .expect("the dir definition exists")
}

/// Calls ListDir over `files` with `args` as the Pack does: the schema
/// check first, then the tool.
fn list(files: &impl Source, args: &Value) -> Result<ToolOutput, ToolError> {
    let definition = definition();
    crate::check::check(&definition.wire_name, &definition.parameters, args)?;
    super::call(files, definition, args)
}

/// Lists `target`, leaving out `ignore`, under the byte cap `cap`.
fn list_capped(
    files: &Files,
    target: &str,
    ignore: &[&str],
    cap: usize,
) -> Result<ToolOutput, ToolError> {
    let request = Request {
        target,
        ignore: ignore
            .iter()
            .map(|pattern| Matcher::new(pattern).expect("the pattern is valid"))
            .collect(),
    };
    super::list_directory(files, "ListDir", &request, cap)
}

/// The text of a successful listing, which must be untrusted output.
fn text_of(result: Result<ToolOutput, ToolError>) -> String {
    let output = result.expect("the listing succeeds");
    assert!(!output.is_trusted(), "{}", output.text());
    output.text().to_owned()
}

/// The message of a failed listing.
fn failure_of(result: Result<ToolOutput, ToolError>) -> String {
    match result {
        Ok(output) => panic!("the listing fails, but it answered {:?}", output.text()),
        Err(error) => error.to_string(),
    }
}
