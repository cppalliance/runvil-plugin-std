//! Unit tests for the `ListDir` tool. This file holds the helpers the
//! topic files share: a tree of files held in memory, the two ways to
//! drive ListDir over it, through its arguments and through a lowered
//! byte cap, and a walk over listings a test writes out by hand.

mod arguments;
mod cap;
mod ignore;
mod tree;

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{DirEntry, FileError, Stat, ToolError, ToolOutput};
use serde_json::Value;

use super::args::Request;
use super::walk::{Failure, Tree, walk};
use crate::fake::block_on;
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
        .iter()
        .find(|definition| definition.segment == "dir")
        .expect("the dir definition exists")
}

/// Calls ListDir over `files` with `args` as the Pack does: the schema
/// check first, then the tool.
fn list(files: &impl Source, args: &Value) -> Result<ToolOutput, ToolError> {
    let definition = definition();
    crate::check::check(&definition.wire_name, &definition.parameters, args)?;
    block_on(super::call(files, definition, args))
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
    block_on(super::list_directory(files, "ListDir", &request, cap))
}

/// A [`Source`] whose listings a test writes out by hand: `list` answers
/// for each directory the walk enters, so a test can hold any kind of
/// entry, fail any listing, and panic on a directory that must not be
/// entered. The walk only lists, so every other operation panics.
struct Lister<F>(F);

#[expect(
    clippy::unused_async_trait_impl,
    reason = "the methods are async for a Host that answers in a later step, and a listing written by hand answers at once"
)]
impl<F> Source for Lister<F>
where
    F: Fn(&str) -> Result<Vec<DirEntry>, FileError>,
{
    async fn stat(&self, path: &str) -> Result<Stat, FileError> {
        panic!("the walk asked what {path} names")
    }

    async fn list(&self, path: &str) -> Result<Vec<DirEntry>, FileError> {
        (self.0)(path)
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, FileError> {
        panic!("the walk read {path}")
    }

    async fn read_prefix(&self, path: &str, _len: u64) -> Result<Vec<u8>, FileError> {
        panic!("the walk read {path}")
    }

    async fn write(&self, path: &str, _bytes: Vec<u8>) -> Result<(), FileError> {
        panic!("the walk wrote {path}")
    }
}

/// Walks `target` over the listings `list` answers with, leaving out
/// `ignore`, under the byte cap `cap`.
fn walk_listed(
    target: &str,
    ignore: &[Matcher],
    cap: usize,
    list: impl Fn(&str) -> Result<Vec<DirEntry>, FileError>,
) -> Result<Tree, Failure> {
    block_on(walk(&Lister(list), target, ignore, cap))
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
