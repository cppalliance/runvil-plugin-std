//! Unit tests for the `Write` tool: what a write leaves in the files,
//! what it answers, and what it refuses before and after it asks for the
//! write.

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{
    FileError, FileErrorKind, FileType, Stat, ToolError, ToolErrorKind, ToolOutput,
};
use serde_json::{Value, json};

use crate::fake::{FakeSource, block_on};
use crate::source::Source;
use crate::spec::{self, Definition};

/// The `write` definition the Pack builds the tool from.
fn definition() -> &'static Definition {
    spec::all()
        .iter()
        .find(|definition| definition.segment == "write")
        .expect("the write definition exists")
}

/// Calls Write over `files` with `args` as the Pack does: the schema
/// check first, then the tool.
fn write(files: &impl Source, args: &Value) -> Result<ToolOutput, ToolError> {
    let definition = definition();
    crate::check::check(&definition.wire_name, &definition.parameters, args)?;
    block_on(super::call(files, definition, args))
}

/// Writes `contents` to `path` in `files`.
fn write_to(files: &impl Source, path: &str, contents: &str) -> Result<ToolOutput, ToolError> {
    write(files, &json!({ "path": path, "contents": contents }))
}

/// The whole contents of the file at `path`.
fn contents(files: &Files, path: &str) -> Vec<u8> {
    files.read(path, 0, u64::MAX).expect("the file is there")
}

/// The text of a successful write, which must be trusted output.
fn text_of(result: Result<ToolOutput, ToolError>) -> String {
    let output = result.expect("the write succeeds");
    assert!(output.is_trusted(), "{}", output.text());
    output.text().to_owned()
}

/// The message of a failed write, which must not be an
/// invalid-arguments failure.
fn failure_of(result: Result<ToolOutput, ToolError>) -> String {
    match result {
        Ok(output) => panic!("the write fails, but it answered {:?}", output.text()),
        Err(error) => {
            assert_eq!(error.kind(), ToolErrorKind::Other, "{error}");
            error.to_string()
        }
    }
}

/// The message of a write whose arguments are refused.
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
fn a_write_makes_a_file_and_every_directory_on_the_way_to_it() {
    let files = Files::new();
    assert_eq!(
        text_of(write_to(&files, "/notes/deep/a.md", "one\ntwo\n")),
        "Wrote 8 bytes."
    );
    assert_eq!(contents(&files, "/notes/deep/a.md"), b"one\ntwo\n");
    assert_eq!(
        files.stat("/notes/deep"),
        Ok(Stat::new(FileType::Directory, 0))
    );
}

#[test]
fn a_write_replaces_the_whole_contents_of_a_file() {
    let files = Files::new().file("/a.md", "a long first version\n");
    assert_eq!(
        text_of(write_to(&files, "/a.md", "short")),
        "Wrote 5 bytes."
    );
    assert_eq!(contents(&files, "/a.md"), b"short");
}

#[test]
fn the_contents_are_written_exactly_as_given() {
    let files = Files::new();
    let text = "caf\u{e9}\r\n\ttab, no final newline";
    assert!(write_to(&files, "/a.txt", text).is_ok());
    assert_eq!(contents(&files, "/a.txt"), text.as_bytes());
}

#[test]
fn the_answer_counts_the_bytes_written_and_not_the_characters() {
    let files = Files::new();
    assert_eq!(text_of(write_to(&files, "/none", "")), "Wrote 0 bytes.");
    assert_eq!(contents(&files, "/none"), b"");
    assert_eq!(text_of(write_to(&files, "/one", "a")), "Wrote 1 byte.");
    assert_eq!(
        text_of(write_to(&files, "/two", "\u{e9}")),
        "Wrote 2 bytes."
    );
}

#[test]
fn a_path_without_a_leading_slash_is_written_from_the_root() {
    let files = Files::new();
    assert!(write_to(&files, "notes/a.md", "x").is_ok());
    assert_eq!(contents(&files, "/notes/a.md"), b"x");
}

#[test]
fn contents_over_the_limit_of_one_write_are_refused_by_name_and_nothing_is_written() {
    let files = Files::new();
    let over = "a".repeat(16_777_217);
    assert_eq!(
        refused(write_to(&files, "/big.txt", &over)),
        "std_Write: `contents` is 16777217 bytes, and one write takes at most 16777216 bytes"
    );
    assert_eq!(files, Files::new());
}

#[test]
fn contents_exactly_at_the_limit_are_written() {
    let files = Files::new();
    let most = "a".repeat(16_777_216);
    assert_eq!(
        text_of(write_to(&files, "/edge.txt", &most)),
        "Wrote 16777216 bytes."
    );
    assert_eq!(
        files.stat("/edge.txt"),
        Ok(Stat::new(FileType::File, 16_777_216))
    );
}

#[test]
fn a_path_that_names_a_directory_fails_with_the_directory_text() {
    let files = Files::new().file("/notes/a.md", "x");
    let before = files.clone();
    assert_eq!(
        failure_of(write_to(&files, "/notes", "y")),
        "Path is a directory, not a file: /notes"
    );
    assert_eq!(
        failure_of(write_to(&files, "/", "y")),
        "Path is a directory, not a file: /"
    );
    assert_eq!(files, before);
}

#[test]
fn a_path_under_a_file_fails_saying_that_a_parent_is_not_a_directory() {
    let files = Files::new().file("/a.md", "x");
    let before = files.clone();
    assert_eq!(
        failure_of(write_to(&files, "/a.md/b.md", "y")),
        "Path has a parent that is not a directory: /a.md/b.md"
    );
    assert_eq!(files, before);
}

#[test]
fn a_failure_echoes_the_path_as_the_model_wrote_it() {
    let files = Files::new().file("/notes/a.md", "x");
    assert_eq!(
        failure_of(write_to(&files, "notes", "y")),
        "Path is a directory, not a file: notes"
    );
    assert_eq!(
        failure_of(write_to(&files, "notes/a.md/b", "y")),
        "Path has a parent that is not a directory: notes/a.md/b"
    );
}

#[test]
fn a_write_the_policy_denies_fails_with_the_policys_own_reason_and_changes_nothing() {
    let reason = "Mode::Ask allows no write of /vault.md";
    let files = Files::new().fails("/vault.md", FileError::new(FileErrorKind::Denied, reason));
    let before = files.clone();
    assert_eq!(failure_of(write_to(&files, "/vault.md", "x")), reason);
    assert_eq!(failure_of(write_to(&files, "/vault.md/in", "x")), reason);
    assert_eq!(files, before);
}

#[test]
fn a_write_that_fails_for_another_reason_is_worded_by_its_kind() {
    let files = Files::new().fails(
        "/disk.md",
        FileError::new(FileErrorKind::Io, "storage full: no space left"),
    );
    assert_eq!(
        failure_of(write_to(&files, "/disk.md", "x")),
        "I/O failure on /disk.md: storage full: no space left"
    );
    // A failure of no named kind reads as the words it came with.
    assert_eq!(
        failure_of(write_to(&FakeSource::default(), "/a.md", "x")),
        "the fake tree takes no change: /a.md"
    );
}

#[test]
fn a_missing_path_or_contents_names_the_parameter() {
    let files = Files::new();
    assert_eq!(
        refused(write(&files, &json!({ "contents": "x" }))),
        "std_Write: `path` is required, and the call left it out"
    );
    assert_eq!(
        refused(write(&files, &json!({ "path": "/a.md" }))),
        "std_Write: `contents` is required, and the call left it out"
    );
    assert_eq!(
        refused(write(&files, &json!({ "path": "/a.md", "contents": 7 }))),
        "std_Write: `contents` must be a JSON string, and the call passed 7"
    );
    assert_eq!(files, Files::new());
}

#[test]
fn an_undeclared_key_is_refused_and_the_parameters_are_named() {
    let files = Files::new();
    assert_eq!(
        refused(write(
            &files,
            &json!({ "path": "/a.md", "contents": "x", "append": true })
        )),
        "std_Write: `append` is not a parameter of this tool. \
         The parameters are `contents` and `path`."
    );
    assert_eq!(files, Files::new());
}

#[test]
fn a_blank_path_is_refused_before_anything_is_written() {
    let files = Files::new();
    for blank in ["", "  ", "\t\n"] {
        assert_eq!(
            refused(write_to(&files, blank, "x")),
            format!(
                "std_Write: `path` is empty or only whitespace, and the call passed \"{blank}\""
            )
        );
    }
    assert_eq!(files, Files::new());
}

#[test]
fn the_refusals_come_in_a_fixed_order() {
    let files = Files::new();
    let over = "a".repeat(16_777_217);
    assert_eq!(
        refused(write(
            &files,
            &json!({ "path": " ", "contents": over, "mode": 7 })
        )),
        "std_Write: `mode` is not a parameter of this tool. \
         The parameters are `contents` and `path`."
    );
    assert_eq!(
        refused(write_to(&files, " ", &over)),
        "std_Write: `path` is empty or only whitespace, and the call passed \" \""
    );
}
