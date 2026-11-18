//! The file operations the tools need, behind a trait.
//!
//! The call's [`ToolCall`] implements it, so every operation a tool
//! performs goes through the Host, and the run's policy applies. Each
//! method is async, because the Host answers a file operation in a later
//! step of the call: awaiting one ends the step. The seam exists so that
//! a test can run a tool's own code over a tree it builds: the SDK's
//! `testing::Files`, or this crate's `FakeSource` for what that tree
//! cannot hold, such as a link, a pipe, and a read that fails after the
//! stat succeeded. Both answer at once, so a test polls a tool's future
//! to its end with no Host around it.

use runvil_plugin_sdk::{DirEntry, FileError, Stat, ToolCall};

/// Where a tool reaches the run's files.
pub(crate) trait Source {
    /// Metadata for `path`, without following a link.
    async fn stat(&self, path: &str) -> Result<Stat, FileError>;

    /// The entries of the directory at `path`, in no promised order.
    async fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError>;

    /// The whole contents of the file at `path`.
    async fn read(&self, path: &str) -> Result<Vec<u8>, FileError>;

    /// At most the first `len` bytes of the file at `path`, for a path
    /// whose size is not known beforehand.
    async fn read_prefix(&self, path: &str, len: u64) -> Result<Vec<u8>, FileError>;

    /// Writes `bytes` as the whole contents of the file at `path`. The
    /// file is made when it is absent, with each directory on the way to
    /// it.
    async fn write(&self, path: &str, bytes: Vec<u8>) -> Result<(), FileError>;
}

impl Source for ToolCall<'_> {
    async fn stat(&self, path: &str) -> Result<Stat, FileError> {
        ToolCall::stat(self, path).await
    }

    async fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError> {
        ToolCall::list(self, path).await
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, FileError> {
        ToolCall::read(self, path).await
    }

    async fn read_prefix(&self, path: &str, len: u64) -> Result<Vec<u8>, FileError> {
        ToolCall::read_range(self, path, 0, len).await
    }

    async fn write(&self, path: &str, bytes: Vec<u8>) -> Result<(), FileError> {
        ToolCall::write(self, path, bytes).await
    }
}

#[cfg(test)]
#[expect(
    clippy::unused_async_trait_impl,
    reason = "the methods are async for a Host that answers in a later step, and a tree held in memory answers at once"
)]
impl Source for runvil_plugin_sdk::testing::Files {
    async fn stat(&self, path: &str) -> Result<Stat, FileError> {
        runvil_plugin_sdk::testing::Files::stat(self, path)
    }

    async fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError> {
        runvil_plugin_sdk::testing::Files::list(self, path)
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, FileError> {
        runvil_plugin_sdk::testing::Files::read(self, path, 0, u64::MAX)
    }

    async fn read_prefix(&self, path: &str, len: u64) -> Result<Vec<u8>, FileError> {
        runvil_plugin_sdk::testing::Files::read(self, path, 0, len)
    }

    async fn write(&self, path: &str, bytes: Vec<u8>) -> Result<(), FileError> {
        runvil_plugin_sdk::testing::Files::write(self, path, bytes)
    }
}
