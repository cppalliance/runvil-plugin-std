//! The seven tool definitions this crate ships, each an embedded JSON file.
//!
//! A file holds the name the model calls the tool by, the description the
//! model reads, and the JSON Schema its arguments must match. The files
//! start from Cursor's definitions of the same tools, edited where a line
//! describes behavior this Pack does not have. `dir.json` is this crate's
//! own. `schemas/EDITS.md` records every edit and why `dir.json` is new.

use runvil_plugin_sdk::{ToolError, ToolErrorKind};
use serde::Deserialize;
use serde_json::Value;

/// One tool's definition: its id segment, and the file's own contents.
#[derive(Debug)]
pub(crate) struct Definition {
    /// The tool's last id segment, such as `read`, which its file's name
    /// gives.
    pub(crate) segment: &'static str,
    /// The name the model calls the tool by.
    pub(crate) wire_name: String,
    /// The description the model reads.
    pub(crate) description: String,
    /// The JSON-Schema `object` the tool's arguments must match.
    pub(crate) parameters: Value,
}

/// One tool's file, as the file itself holds it.
#[derive(Debug, Deserialize)]
struct File {
    /// The name the model calls the tool by.
    wire_name: String,
    /// The description the model reads.
    description: String,
    /// The JSON-Schema `object` the tool's arguments must match.
    parameters: Value,
}

/// Each tool's id segment beside the source of the file it reads.
const SOURCES: [(&str, &str); 7] = [
    ("read", include_str!("../schemas/read.json")),
    ("write", include_str!("../schemas/write.json")),
    ("replace", include_str!("../schemas/replace.json")),
    ("rm", include_str!("../schemas/rm.json")),
    ("glob", include_str!("../schemas/glob.json")),
    ("grep", include_str!("../schemas/grep.json")),
    ("dir", include_str!("../schemas/dir.json")),
];

/// Every definition, parsed from the embedded files, in [`SOURCES`]
/// order. The Plugin calls this once, when it loads, and keeps the
/// result in its instance.
///
/// # Errors
/// Returns a [`Backend`](ToolErrorKind::Backend) error naming the tool
/// whose file does not parse. The files are embedded, so this can only
/// happen after an edit.
pub(crate) fn load() -> Result<Vec<Definition>, ToolError> {
    parse(&SOURCES)
}

/// The definition of each of `sources`, an id segment beside the text of
/// its file, in order.
fn parse(sources: &[(&'static str, &str)]) -> Result<Vec<Definition>, ToolError> {
    sources
        .iter()
        .map(|(segment, source)| {
            let file: File = serde_json::from_str(source).map_err(|error| {
                ToolError::new(
                    ToolErrorKind::Backend,
                    format!("std could not read the definition of its {segment} tool: {error}"),
                )
            })?;
            Ok(Definition {
                segment,
                wire_name: file.wire_name,
                description: fill(segment, &file.description),
                parameters: file.parameters,
            })
        })
        .collect()
}

/// `description` with the tool's `{...}` placeholders filled from the
/// constants its code uses, so the text the model reads and the limit the
/// code enforces cannot drift apart.
///
/// A tool without placeholders keeps its text as it is.
fn fill(segment: &str, description: &str) -> String {
    match segment {
        "read" => description
            .replace("{byte_cap}", &crate::read::BYTE_CAP.to_string())
            .replace("{file_ceiling}", &crate::text::MAX_FILE_BYTES.to_string()),
        "dir" => description.replace("{body_cap}", &crate::text::BODY_CAP.to_string()),
        "grep" => description
            .replace("{body_cap}", &crate::text::BODY_CAP.to_string())
            .replace("{file_ceiling}", &crate::text::MAX_FILE_BYTES.to_string()),
        _ => description.to_owned(),
    }
}

/// Every definition, parsed once for the tests of every module.
///
/// # Panics
/// Panics when an embedded file does not parse.
#[cfg(test)]
pub(crate) fn all() -> &'static [Definition] {
    static DEFINITIONS: std::sync::LazyLock<Vec<Definition>> =
        std::sync::LazyLock::new(|| load().expect("every embedded definition parses"));
    &DEFINITIONS
}

#[cfg(test)]
#[path = "spec-tests.rs"]
mod tests;
