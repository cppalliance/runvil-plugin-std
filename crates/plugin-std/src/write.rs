//! The `Write` tool: makes a file, or replaces the whole contents of one.
//!
//! Write hands the call's contents to the Host as one write through the
//! call's [`Source`], so the run's policy applies. The Host makes the
//! file when it is absent, with each directory on the way to it. Write
//! asks for nothing else: it takes no stat and no read first, so what
//! was at the path is gone when the write succeeds. Contents over
//! [`RV_MAX_WRITE`] bytes, the most one write takes, are refused before
//! the Host is asked for anything. The answer is fixed text and a count,
//! so it is trusted output.

use runvil_plugin_sdk::sys::RV_MAX_WRITE;
use runvil_plugin_sdk::{ToolError, ToolErrorKind, ToolOutput};
use serde_json::Value;

use crate::check::{make_invalid_arguments_error, plural, refuse_blank, refuse_unknown};
use crate::refusal::write_refusal;
use crate::source::Source;
use crate::spec::Definition;

/// Runs one `Write` call whose arguments already passed the shared schema
/// check.
///
/// The order of the refusals is fixed: an undeclared key, then a blank
/// path, then contents over the limit, so a call that breaks two rules
/// always hears about the same one.
///
/// # Errors
/// Returns an invalid-arguments [`ToolError`] naming the parameter for a
/// bad argument, and a message error for a write the Host fails, worded
/// for a model to read.
pub(crate) async fn call(
    source: &impl Source,
    definition: &Definition,
    args: &Value,
) -> Result<ToolOutput, ToolError> {
    let tool = definition.wire_name.as_str();
    refuse_unknown(tool, &definition.parameters, args)?;
    let path = args.get("path").and_then(Value::as_str).unwrap_or_default();
    refuse_blank(tool, "path", path)?;
    let contents = args
        .get("contents")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if contents.len() > RV_MAX_WRITE {
        return Err(make_invalid_arguments_error(
            tool,
            format!(
                "`contents` is {} bytes, and one write takes at most {RV_MAX_WRITE} bytes",
                contents.len()
            ),
        ));
    }
    source
        .write(path, contents.as_bytes().to_vec())
        .await
        .map_err(|error| ToolError::new(ToolErrorKind::Other, write_refusal(tool, path, &error)))?;
    Ok(ToolOutput::trusted(format!(
        "Wrote {}.",
        plural(contents.len(), "byte")
    )))
}

#[cfg(test)]
#[path = "write-tests.rs"]
mod tests;
