//! Unit tests for the `Delete` tool: what a call removes, what it
//! refuses, what it answers, and how a failure of the Host reads.

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{FileError, FileErrorKind, FileType, ToolError, ToolErrorKind, ToolOutput};
use serde_json::{Value, json};

use super::deleted;
use crate::fake::{FakeSource, Node, block_on};
use crate::source::Source;
use crate::spec::{self, Definition};

/// The `rm` definition the Pack builds the tool from.
fn definition() -> &'static Definition {
    spec::all()
        .iter()
        .find(|definition| definition.segment == "rm")
        .expect("the rm definition exists")
}

/// Calls Delete over `files` with `args` as the Pack does: the schema
/// check first, then the tool.
fn delete(files: &impl Source, args: &Value) -> Result<ToolOutput, ToolError> {
    let definition = definition();
    crate::check::check(&definition.wire_name, &definition.parameters, args)?;
    block_on(super::call(files, definition, args))
}

/// Deletes what is at `path` in `files`.
fn delete_at(files: &impl Source, path: &str) -> Result<ToolOutput, ToolError> {
    delete(files, &json!({ "path": path }))
}

/// The text of a successful delete, which must be trusted output.
fn text_of(result: Result<ToolOutput, ToolError>) -> String {
    let output = result.expect("the delete succeeds");
    assert!(output.is_trusted(), "{}", output.text());
    output.text().to_owned()
}

/// The message of a failed delete, which must not be an
/// invalid-arguments failure.
fn failure_of(result: Result<ToolOutput, ToolError>) -> String {
    match result {
        Ok(output) => panic!("the delete fails, but it answered {:?}", output.text()),
        Err(error) => {
            assert_eq!(error.kind(), ToolErrorKind::Other, "{error}");
            error.to_string()
        }
    }
}

/// The message of a delete whose arguments are refused.
fn refused(result: Result<ToolOutput, ToolError>) -> String {
    match result {
        Ok(output) => panic!("the call is refused, but it answered {:?}", output.text()),
        Err(error) => {
            assert_eq!(error.kind(), ToolErrorKind::InvalidArguments, "{error}");
            error.to_string()
        }
    }
}

#[test]
fn a_delete_removes_the_file_and_leaves_what_is_beside_it() {
    let files = Files::new()
        .file("/notes/a.md", "one")
        .file("/notes/b.md", "two");
    assert_eq!(
        text_of(delete_at(&files, "/notes/a.md")),
        "Deleted the file."
    );
    assert_eq!(files, Files::new().file("/notes/b.md", "two"));
}

#[test]
fn the_directory_that_held_the_file_stays() {
    let files = Files::new().file("/notes/a.md", "one");
    assert!(delete_at(&files, "/notes/a.md").is_ok());
    assert_eq!(files, Files::new().dir("/notes"));
}

#[test]
fn a_path_without_a_leading_slash_is_deleted_from_the_root() {
    let files = Files::new().file("/notes/a.md", "one");
    assert!(delete_at(&files, "notes/a.md").is_ok());
    assert_eq!(files, Files::new().dir("/notes"));
}

#[test]
fn a_missing_path_fails_with_the_not_found_text() {
    let files = Files::new().file("/notes/a.md", "one");
    let before = files.clone();
    assert_eq!(
        failure_of(delete_at(&files, "/nope.md")),
        "File not found: /nope.md"
    );
    assert_eq!(
        failure_of(delete_at(&files, "notes/gone.md")),
        "File not found: notes/gone.md"
    );
    assert_eq!(files, before);
}

#[test]
fn a_directory_is_refused_with_a_sentence_and_stays() {
    let files = Files::new().file("/full/a.md", "one").dir("/empty");
    let before = files.clone();
    for directory in ["/full", "/empty", "empty", "/"] {
        assert_eq!(
            failure_of(delete_at(&files, directory)),
            format!("Path is a directory, not a file: {directory}")
        );
    }
    assert_eq!(files, before);
}

#[test]
fn a_file_a_link_and_a_pipe_all_get_as_far_as_the_removal() {
    // The fake takes no change, so this text shows that the call passed
    // the stat and asked for the removal.
    let source = FakeSource::default()
        .file("/file", "x")
        .with("/filelink", Node::LinkToFile(b"x".to_vec()))
        .with("/dirlink", Node::Link)
        .with("/pipe", Node::Pipe);
    for path in ["/file", "/filelink", "/dirlink", "/pipe"] {
        assert_eq!(
            failure_of(delete_at(&source, path)),
            format!("the fake tree takes no change: {path}")
        );
    }
}

#[test]
fn the_answer_says_that_what_a_link_leads_to_stays() {
    assert_eq!(deleted(FileType::File), "Deleted the file.");
    assert_eq!(deleted(FileType::Other), "Deleted the file.");
    assert_eq!(
        deleted(FileType::Symlink),
        "Deleted the link. What it leads to stays."
    );
}

#[test]
fn a_stat_the_policy_denies_fails_with_the_policys_own_reason_and_removes_nothing() {
    let reason = "Mode::Ask allows no change of /vault.md";
    let files = Files::new().fails("/vault.md", FileError::new(FileErrorKind::Denied, reason));
    let before = files.clone();
    assert_eq!(failure_of(delete_at(&files, "/vault.md")), reason);
    assert_eq!(files, before);
}

#[test]
fn a_removal_the_host_fails_after_the_stat_succeeded_is_worded_by_its_kind() {
    let guarded = |kind, text: &str| Node::Guarded(b"x".to_vec(), FileError::new(kind, text));
    let reason = "Mode::Ask allows no change of /ask.md";
    let source = FakeSource::default()
        .with("/ask.md", guarded(FileErrorKind::Denied, reason))
        .with(
            "/disk.md",
            guarded(FileErrorKind::Io, "permission denied: access denied"),
        )
        .with(
            "/gone.md",
            guarded(FileErrorKind::NotFound, "not found: /gone.md"),
        );
    assert_eq!(failure_of(delete_at(&source, "/ask.md")), reason);
    assert_eq!(
        failure_of(delete_at(&source, "/disk.md")),
        "I/O failure on /disk.md: permission denied: access denied"
    );
    assert_eq!(
        failure_of(delete_at(&source, "/gone.md")),
        "File not found: /gone.md"
    );
}

#[test]
fn a_missing_path_argument_or_one_of_the_wrong_type_names_the_parameter() {
    let files = Files::new();
    assert_eq!(
        refused(delete(&files, &json!({}))),
        "std_Delete: `path` is required, and the call left it out"
    );
    assert_eq!(
        refused(delete(&files, &json!({ "path": ["/a.md"] }))),
        "std_Delete: `path` must be a JSON string, and the call passed [\"/a.md\"]"
    );
}

#[test]
fn an_undeclared_key_is_refused_before_a_blank_path() {
    let files = Files::new().file("/a.md", "one");
    let before = files.clone();
    assert_eq!(
        refused(delete(&files, &json!({ "path": " ", "recursive": true }))),
        "std_Delete: `recursive` is not a parameter of this tool. The parameters are `path`."
    );
    for blank in ["", "  ", "\t\n"] {
        assert_eq!(
            refused(delete_at(&files, blank)),
            format!(
                "std_Delete: `path` is empty or only whitespace, and the call passed \"{blank}\""
            )
        );
    }
    assert_eq!(files, before);
}
