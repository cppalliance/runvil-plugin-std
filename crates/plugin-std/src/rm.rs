//! The `Delete` tool: removes one file.
//!
//! Delete asks what the path names and then asks for its removal, both
//! through the call's [`Source`], so the run's policy applies. The stat
//! comes first because the removal the Host offers also takes a
//! directory with no entries, and Delete removes no directory: it
//! refuses one with a sentence. A link goes as a link, so what it leads
//! to stays. The answer is fixed text, so it is trusted output.

use runvil_plugin_sdk::{FileError, FileType, ToolError, ToolErrorKind, ToolOutput};
use serde_json::Value;

use crate::check::{refuse_blank, refuse_unknown};
use crate::refusal::{is_a_directory, refusal};
use crate::source::Source;
use crate::spec::Definition;

/// Runs one `Delete` call whose arguments already passed the shared
/// schema check.
///
/// # Errors
/// Returns an invalid-arguments [`ToolError`] naming the parameter for a
/// bad argument, and a message error for any other failure, worded for a
/// model to read: nothing is at the path, the path names a directory, or
/// the Host fails the stat or the removal.
pub(crate) async fn call(
    source: &impl Source,
    definition: &Definition,
    args: &Value,
) -> Result<ToolOutput, ToolError> {
    let tool = definition.wire_name.as_str();
    refuse_unknown(tool, &definition.parameters, args)?;
    let path = args.get("path").and_then(Value::as_str).unwrap_or_default();
    refuse_blank(tool, "path", path)?;
    let message = |text: String| ToolError::new(ToolErrorKind::Other, text);
    let failed = |error: FileError| message(refusal(tool, path, &error));
    let stat = source.stat(path).await.map_err(failed)?;
    if stat.file_type == FileType::Directory {
        return Err(message(is_a_directory(path)));
    }
    source.remove(path).await.map_err(failed)?;
    Ok(ToolOutput::trusted(deleted(stat.file_type)))
}

/// What a call answers after it removed something of `file_type`. A link
/// goes as a link, so the answer says that what it leads to stays. A
/// pipe, a socket, or a device goes as a file does.
fn deleted(file_type: FileType) -> &'static str {
    if file_type == FileType::Symlink {
        "Deleted the link. What it leads to stays."
    } else {
        "Deleted the file."
    }
}

#[cfg(test)]
#[path = "rm-tests.rs"]
mod tests;
