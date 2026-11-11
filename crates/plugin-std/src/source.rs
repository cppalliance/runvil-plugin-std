//! The file operations the tools need, behind a trait.
//!
//! The call's [`ToolCall`] implements it, so every operation a tool
//! performs goes through the Host, and the run's mode and policy apply.
//! The seam exists so that a test can run a tool's own code over a tree
//! it builds, and can supply what a plain tree cannot hold: a symbolic
//! link, a name with a control character, an over-size entry, a denied
//! read, and a claim conflict.

use runvil_plugin_sdk::{DirEntry, FileError, Stat, ToolCall};

/// Where a tool reads the run's files from.
pub(crate) trait Source {
    /// Metadata for `path`, without following a link.
    fn stat(&self, path: &str) -> Result<Stat, FileError>;

    /// The entries of the directory at `path`, in no promised order.
    fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError>;

    /// The whole contents of the file at `path`.
    fn read(&self, path: &str) -> Result<Vec<u8>, FileError>;

    /// At most the first `len` bytes of the file at `path`, for a path
    /// whose size is not known beforehand.
    fn read_prefix(&self, path: &str, len: u64) -> Result<Vec<u8>, FileError>;
}

impl Source for ToolCall<'_> {
    fn stat(&self, path: &str) -> Result<Stat, FileError> {
        ToolCall::stat(self, path)
    }

    fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError> {
        ToolCall::list(self, path)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, FileError> {
        ToolCall::read(self, path)
    }

    fn read_prefix(&self, path: &str, len: u64) -> Result<Vec<u8>, FileError> {
        ToolCall::read_range(self, path, 0, len)
    }
}

#[cfg(test)]
impl Source for runvil_plugin_sdk::testing::Files {
    fn stat(&self, path: &str) -> Result<Stat, FileError> {
        runvil_plugin_sdk::testing::Files::stat(self, path)
    }

    fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError> {
        runvil_plugin_sdk::testing::Files::list(self, path)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, FileError> {
        runvil_plugin_sdk::testing::Files::read(self, path, 0, u64::MAX)
    }

    fn read_prefix(&self, path: &str, len: u64) -> Result<Vec<u8>, FileError> {
        runvil_plugin_sdk::testing::Files::read(self, path, 0, len)
    }
}
