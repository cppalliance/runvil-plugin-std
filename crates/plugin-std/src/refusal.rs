//! The sentence a model reads when a file operation fails.
//!
//! Every file tool turns a [`FileError`] into one of these, so the same
//! failure reads the same in each tool. A path this module writes into a
//! sentence is the path as the model wrote it, never the path the Host
//! resolved it to, so a relative `notes/a.md` is not echoed back as
//! `/notes/a.md`. The Host's own words, which some sentences quote, may
//! name the resolved path.

use runvil_plugin_sdk::{FileError, FileErrorKind};

/// The model-readable sentence for `error`, from the tool called `tool`
/// (the name the model uses) on `path` (as the model wrote it).
///
/// A sentence names no tool other than `tool`. A read the run's policy
/// denies, and a failure the contract has no kind for, read as the Host's
/// own words alone. An I/O failure reads as the path and then the Host's
/// words.
pub(crate) fn refusal(tool: &str, path: &str, error: &FileError) -> String {
    match error.kind() {
        FileErrorKind::NotFound => format!("File not found: {path}"),
        FileErrorKind::IsADirectory => format!("Path is a directory, not a file: {path}"),
        FileErrorKind::NotADirectory => not_a_directory(path),
        FileErrorKind::Conflict => format!(
            "{path} was changed, or is being changed, by another call in this round, so it \
             cannot be read in this round. Call {tool} again in your next message."
        ),
        FileErrorKind::Io => format!("I/O failure on {path}: {}", error.text()),
        _ => error.text().to_owned(),
    }
}

/// The sentence for a root the model named that does not exist.
///
/// A tool that starts from a directory words its own missing root, because
/// the `NotFound` sentence above says a file was not found.
pub(crate) fn missing_root(path: &str) -> String {
    format!("Path does not exist: {path}")
}

/// The sentence for a path that names something other than a directory
/// where a directory was needed.
pub(crate) fn not_a_directory(path: &str) -> String {
    format!("Path is not a directory: {path}")
}

/// The sentence for a path that is neither a file nor a directory, such
/// as a pipe or a device.
pub(crate) fn neither_file_nor_directory(path: &str) -> String {
    format!("Path is neither a file nor a directory: {path}")
}

#[cfg(test)]
mod tests;
