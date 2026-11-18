//! What the tests of every tool share: [`FakeSource`], a [`Source`] they
//! build by hand, and [`block_on`], which runs a tool's future to its
//! end.
//!
//! The fake holds what the fake Host's tree cannot: a link to a directory
//! or to a file, a pipe, a file whose listed size differs from its bytes,
//! a directory or a file that fails only when it is listed or read, and a
//! file that fails only a change. A name may hold any
//! character, a control character among them.
//!
//! Every node sits directly under `/`, or under a [`Node::Link`], which
//! lists as a directory. The tree never changes: it fails every change,
//! with the error of a [`Node::Guarded`] where the path names one.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use runvil_plugin_sdk::{DirEntry, FileError, FileErrorKind, FileType, Stat};

use crate::source::Source;

/// Runs `future` to its output, polling it again after each `Pending`.
///
/// A tool's future waits on no waker. Over a tree held in memory each
/// file operation answers at once, so the only `Pending` is a pause,
/// which the next poll ends.
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

/// One node of the fake tree.
#[derive(Debug, Clone)]
pub(crate) enum Node {
    /// A file with these bytes.
    File(Vec<u8>),
    /// A symbolic link to a directory.
    Link,
    /// A symbolic link to a file with these bytes.
    LinkToFile(Vec<u8>),
    /// A symbolic link that fails with this error when it is listed or
    /// read.
    LinkFails(FileError),
    /// A pipe.
    Pipe,
    /// A file whose listing says it holds this many bytes, and whose
    /// bytes are the second field.
    Sized(u64, Vec<u8>),
    /// A directory whose listing fails with this error.
    ListFails(FileError),
    /// A file whose read fails with this error.
    ReadFails(FileError),
    /// A file with these bytes that fails each change with this error, as
    /// a file that the run's policy lets a tool read and not change.
    Guarded(Vec<u8>, FileError),
}

/// A tree of nodes by absolute path, with `/` always a directory.
#[derive(Debug, Default)]
pub(crate) struct FakeSource {
    /// Every node but the root.
    nodes: BTreeMap<String, Node>,
    /// The most bytes that each read asked for, in the order of the reads.
    asked: RefCell<Vec<u64>>,
}

impl FakeSource {
    /// The most bytes that each read of the tree asked for, in the order
    /// of the reads.
    pub(crate) fn asked(&self) -> Vec<u64> {
        self.asked.borrow().clone()
    }

    /// The tree with `node` added at `path`.
    pub(crate) fn with(mut self, path: &str, node: Node) -> FakeSource {
        self.nodes.insert(path.to_owned(), node);
        self
    }

    /// The tree with the text file `text` at `path`.
    pub(crate) fn file(self, path: &str, text: &str) -> FakeSource {
        self.with(path, Node::File(text.as_bytes().to_vec()))
    }

    /// The metadata of `node`.
    fn stat_of(node: &Node) -> Stat {
        match node {
            Node::File(bytes) | Node::Guarded(bytes, _) => {
                Stat::new(FileType::File, bytes.len() as u64)
            }
            Node::Sized(size, _) => Stat::new(FileType::File, *size),
            Node::ReadFails(_) => Stat::new(FileType::File, 1),
            Node::ListFails(_) => Stat::new(FileType::Directory, 0),
            Node::Link | Node::LinkToFile(_) | Node::LinkFails(_) => {
                Stat::new(FileType::Symlink, 0)
            }
            Node::Pipe => Stat::new(FileType::Other, 0),
        }
    }

    /// The error for a path that is not there.
    fn missing(path: &str) -> FileError {
        FileError::new(FileErrorKind::NotFound, format!("not found: {path}"))
    }

    /// Every byte of the file at `path`, as a read of the whole file
    /// would give them.
    fn bytes_at(&self, path: &str) -> Result<Vec<u8>, FileError> {
        match self.nodes.get(path) {
            Some(
                Node::File(bytes)
                | Node::Sized(_, bytes)
                | Node::LinkToFile(bytes)
                | Node::Guarded(bytes, _),
            ) => Ok(bytes.clone()),
            Some(Node::ReadFails(error) | Node::LinkFails(error)) => Err(error.clone()),
            Some(Node::Link | Node::ListFails(_)) => Err(FileError::new(
                FileErrorKind::IsADirectory,
                format!("is a directory: {path}"),
            )),
            Some(Node::Pipe) | None => Err(Self::missing(path)),
        }
    }

    /// The error of a change to `path`, which the tree never takes: the
    /// error of a [`Node::Guarded`], and a fixed one for any other path.
    fn refused(&self, path: &str) -> FileError {
        match self.nodes.get(path) {
            Some(Node::Guarded(_, error)) => error.clone(),
            _ => FileError::new(
                FileErrorKind::Other,
                format!("the fake tree takes no change: {path}"),
            ),
        }
    }
}

/// The directory that holds `path`, and the name it has there.
fn split(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some(("", name)) => ("/", name),
        Some((parent, name)) => (parent, name),
        None => ("/", path),
    }
}

#[expect(
    clippy::unused_async_trait_impl,
    reason = "the methods are async for a Host that answers in a later step, and a tree held in memory answers at once"
)]
impl Source for FakeSource {
    async fn stat(&self, path: &str) -> Result<Stat, FileError> {
        if path == "/" {
            return Ok(Stat::new(FileType::Directory, 0));
        }
        self.nodes
            .get(path)
            .map(Self::stat_of)
            .ok_or_else(|| Self::missing(path))
    }

    async fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError> {
        if path != "/" {
            match self.nodes.get(path) {
                None => return Err(Self::missing(path)),
                Some(Node::ListFails(error) | Node::LinkFails(error)) => {
                    return Err(error.clone());
                }
                Some(Node::Link) => {}
                Some(_) => {
                    return Err(FileError::new(
                        FileErrorKind::NotADirectory,
                        format!("not a directory: {path}"),
                    ));
                }
            }
        }
        Ok(self
            .nodes
            .iter()
            .filter(|(child, _)| split(child).0 == path)
            .map(|(child, node)| DirEntry::new(split(child).1, Self::stat_of(node)))
            .collect())
    }

    async fn read_prefix(&self, path: &str, len: u64) -> Result<Vec<u8>, FileError> {
        self.asked.borrow_mut().push(len);
        let mut bytes = self.bytes_at(path)?;
        bytes.truncate(usize::try_from(len).unwrap_or(usize::MAX));
        Ok(bytes)
    }

    async fn write(&self, path: &str, _bytes: Vec<u8>) -> Result<(), FileError> {
        Err(self.refused(path))
    }

    async fn remove(&self, path: &str) -> Result<(), FileError> {
        Err(self.refused(path))
    }
}
