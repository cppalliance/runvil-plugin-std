//! What the directory-walking tools share: the order of a walk, the one
//! name a walk skips, and the names a walk never prints.
//!
//! ListDir and Grep both walk a directory tree, and the order they visit
//! entries in is a promise to the model: it is the order the Engine's
//! `Access::glob` gives, so a walk and a glob over the same tree list the
//! same paths in the same sequence. Both tools take all three from here,
//! so the two cannot drift apart.

use runvil_plugin_sdk::{DirEntry, FileType};

/// The name of the entry a walk skips, a file or a directory, at any
/// depth.
pub(crate) const GIT: &str = ".git";

/// Whether `name` holds a control character: one below U+0020, or
/// U+007F.
///
/// A walk never prints such a name, because a line feed in it would forge
/// a line of the answer. It counts the entry instead, and does not enter
/// it.
pub(crate) fn has_control(name: &str) -> bool {
    name.chars().any(|c| c < ' ' || c == '\u{7f}')
}

/// `entries` in the order a walk visits them: by name, with `/` added to
/// a directory's name.
///
/// A listing comes in no promised order. This is the order the Engine's
/// `Access::glob` gives, which sorts full path strings bytewise.
pub(crate) fn sorted(mut entries: Vec<DirEntry>) -> Vec<DirEntry> {
    entries.sort_by_cached_key(|entry| {
        if matches!(entry.stat.file_type, FileType::Directory) {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        }
    });
    entries
}

#[cfg(test)]
mod tests;
