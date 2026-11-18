//! The `StrReplace` tool: replaces a text in a file with another text.
//!
//! StrReplace reads the file as Read does, through the call's
//! [`Source`], so the run's policy applies and the same paths are
//! refused: a directory, a special file, and a file over 16 MiB. The file
//! must hold UTF-8. Nothing is decoded, and nothing outside the matches
//! changes, so a byte order mark and the line endings stay as they are.
//! The text to replace must occur exactly once, unless `replace_all` is
//! set, and a failure states how many times it occurs.
//!
//! The edit is a read and then a write, each a request to the Host of its
//! own. A change that another task makes to the file between the two is
//! lost, because the write holds the whole file as the read found it.
//!
//! The answer is fixed text and a count, so it is trusted output.

use runvil_plugin_sdk::sys::RV_MAX_WRITE;
use runvil_plugin_sdk::{ToolError, ToolErrorKind, ToolOutput};
use serde_json::Value;

use crate::check::{make_invalid_arguments_error, plural, refuse_blank, refuse_unknown};
use crate::refusal::write_refusal;
use crate::source::Source;
use crate::spec::Definition;

/// What a call asks StrReplace for.
#[derive(Debug)]
struct Request<'a> {
    /// The path as the model wrote it.
    path: &'a str,
    /// The text to replace, which is not empty.
    old: &'a str,
    /// The text that takes its place, which differs from `old`.
    new: &'a str,
    /// Whether the new text goes in at every occurrence. Otherwise the
    /// text must occur exactly once.
    all: bool,
}

/// Runs one `StrReplace` call whose arguments already passed the shared
/// schema check.
///
/// # Errors
/// Returns an invalid-arguments [`ToolError`] naming the parameter for a
/// bad argument, and a message error for any other failure, worded for a
/// model to read. Every refusal comes before the write, so it leaves the
/// file as it was.
pub(crate) async fn call(
    source: &impl Source,
    definition: &Definition,
    args: &Value,
) -> Result<ToolOutput, ToolError> {
    let request = parse(definition, args)?;
    let tool = definition.wire_name.as_str();
    let path = request.path;
    let message = |text: String| ToolError::new(ToolErrorKind::Other, text);
    let bytes = crate::read::load(source, tool, path)
        .await
        .map_err(message)?;
    let Ok(text) = String::from_utf8(bytes) else {
        return Err(message(format!(
            "File is not UTF-8 text, so it cannot be edited: {path}"
        )));
    };
    let count = text.matches(request.old).count();
    if count == 0 || (count > 1 && !request.all) {
        return Err(message(miscount(path, count, request.all)));
    }
    let edited = text.replace(request.old, request.new);
    if edited.len() > RV_MAX_WRITE {
        return Err(message(format!(
            "The edit would leave {path} with {} bytes, and one write takes at most \
             {RV_MAX_WRITE} bytes.",
            edited.len()
        )));
    }
    source
        .write(path, edited.into_bytes())
        .await
        .map_err(|error| message(write_refusal(tool, path, &error)))?;
    Ok(ToolOutput::trusted(format!(
        "Made {}.",
        plural(count, "replacement")
    )))
}

/// Reads the call's arguments after the shared schema check.
///
/// The order is fixed: an undeclared key, then a blank path, then an
/// empty `old_string`, then a `new_string` equal to it, so a call that
/// breaks two rules always hears about the same one.
///
/// # Errors
/// Returns an invalid-arguments [`ToolError`] naming the parameter.
fn parse<'a>(definition: &Definition, args: &'a Value) -> Result<Request<'a>, ToolError> {
    let tool = definition.wire_name.as_str();
    refuse_unknown(tool, &definition.parameters, args)?;
    let text = |name: &str| args.get(name).and_then(Value::as_str).unwrap_or_default();
    let path = text("path");
    refuse_blank(tool, "path", path)?;
    let (old, new) = (text("old_string"), text("new_string"));
    if old.is_empty() {
        return Err(make_invalid_arguments_error(
            tool,
            "`old_string` is empty. Pass the text to replace.",
        ));
    }
    if old == new {
        return Err(make_invalid_arguments_error(
            tool,
            "`new_string` is the same as `old_string`, so the call would change nothing",
        ));
    }
    let all = args
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(Request {
        path,
        old,
        new,
        all,
    })
}

/// The sentence for a text to replace that occurs `count` times in the
/// file at `path`, where the call needs it once, or at least once when it
/// replaces `all`.
fn miscount(path: &str, count: usize, all: bool) -> String {
    let occurs = format!("`old_string` occurs {} in {path}", plural(count, "time"));
    if count == 0 {
        let needed = if all { "at least once" } else { "exactly once" };
        format!(
            "{occurs}, and it must occur {needed}. It is matched exactly, so whitespace and \
             line endings count."
        )
    } else {
        format!(
            "{occurs}, and it must occur exactly once. Add text around it until it names one \
             occurrence, or set `replace_all` to replace every one."
        )
    }
}

#[cfg(test)]
#[path = "replace-tests.rs"]
mod tests;
