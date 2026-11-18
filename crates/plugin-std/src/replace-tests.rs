//! Unit tests for the `StrReplace` tool: what an edit leaves in the file,
//! what it answers, how it counts the text to replace, and what it
//! refuses before it reads and before it writes.

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{
    FileError, FileErrorKind, FileType, Stat, ToolError, ToolErrorKind, ToolOutput,
};
use serde_json::{Value, json};

use super::edited_len;
use crate::fake::{FakeSource, Node, block_on};
use crate::source::Source;
use crate::spec::{self, Definition};

/// The `replace` definition the Pack builds the tool from.
fn definition() -> &'static Definition {
    spec::all()
        .iter()
        .find(|definition| definition.segment == "replace")
        .expect("the replace definition exists")
}

/// Calls StrReplace over `files` with `args` as the Pack does: the schema
/// check first, then the tool.
fn replace(files: &impl Source, args: &Value) -> Result<ToolOutput, ToolError> {
    let definition = definition();
    crate::check::check(&definition.wire_name, &definition.parameters, args)?;
    block_on(super::call(files, definition, args))
}

/// Replaces `old` with `new` in the file at `path`, where `old` must
/// occur once.
fn edit(files: &impl Source, path: &str, old: &str, new: &str) -> Result<ToolOutput, ToolError> {
    replace(
        files,
        &json!({ "path": path, "old_string": old, "new_string": new }),
    )
}

/// Replaces every `old` with `new` in the file at `path`.
fn edit_all(
    files: &impl Source,
    path: &str,
    old: &str,
    new: &str,
) -> Result<ToolOutput, ToolError> {
    replace(
        files,
        &json!({ "path": path, "old_string": old, "new_string": new, "replace_all": true }),
    )
}

/// The whole contents of the file at `path`.
fn contents(files: &Files, path: &str) -> Vec<u8> {
    files.read(path, 0, u64::MAX).expect("the file is there")
}

/// The text of a successful edit, which must be trusted output.
fn text_of(result: Result<ToolOutput, ToolError>) -> String {
    let output = result.expect("the edit succeeds");
    assert!(output.is_trusted(), "{}", output.text());
    output.text().to_owned()
}

/// The message of a failed edit, which must not be an invalid-arguments
/// failure.
fn failure_of(result: Result<ToolOutput, ToolError>) -> String {
    match result {
        Ok(output) => panic!("the edit fails, but it answered {:?}", output.text()),
        Err(error) => {
            assert_eq!(error.kind(), ToolErrorKind::Other, "{error}");
            error.to_string()
        }
    }
}

/// The message of an edit whose arguments are refused.
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
fn the_new_text_takes_the_place_of_the_one_occurrence_and_the_rest_of_the_file_stays() {
    let files = Files::new().file("/a.md", "one\ntwo\nthree\n");
    assert_eq!(
        text_of(edit(&files, "/a.md", "two", "2")),
        "Made 1 replacement."
    );
    assert_eq!(contents(&files, "/a.md"), b"one\n2\nthree\n");
}

#[test]
fn an_old_string_may_span_lines_and_the_new_string_may_be_empty() {
    let files = Files::new().file("/a.md", "one\ntwo\nthree\n");
    assert!(edit(&files, "/a.md", "one\ntwo\n", "").is_ok());
    assert_eq!(contents(&files, "/a.md"), b"three\n");
}

#[test]
fn replace_all_replaces_every_occurrence_and_counts_them() {
    let files = Files::new().file("/a.rs", "let x = x + x;\n");
    assert_eq!(
        text_of(edit_all(&files, "/a.rs", "x", "total")),
        "Made 3 replacements."
    );
    assert_eq!(contents(&files, "/a.rs"), b"let total = total + total;\n");
}

#[test]
fn replace_all_with_one_occurrence_replaces_it() {
    let files = Files::new().file("/a.md", "one two\n");
    assert_eq!(
        text_of(edit_all(&files, "/a.md", "two", "2")),
        "Made 1 replacement."
    );
    assert_eq!(contents(&files, "/a.md"), b"one 2\n");
}

#[test]
fn an_old_string_that_occurs_more_than_once_fails_with_the_count_and_changes_nothing() {
    let files = Files::new().file("/a.rs", "let x = x + x;\n");
    let before = files.clone();
    assert_eq!(
        failure_of(edit(&files, "/a.rs", "x", "total")),
        "`old_string` occurs 3 times in /a.rs, and it must occur exactly once. Add text around \
         it until it names one occurrence, or set `replace_all` to replace every one."
    );
    assert_eq!(files, before);
}

#[test]
fn an_old_string_that_is_absent_fails_with_the_count_and_changes_nothing() {
    let files = Files::new().file("/a.md", "one\ntwo\n");
    let before = files.clone();
    assert_eq!(
        failure_of(edit(&files, "/a.md", "three", "3")),
        "`old_string` occurs 0 times in /a.md, and it must occur exactly once. It is matched \
         exactly, so whitespace and line endings count."
    );
    assert_eq!(
        failure_of(edit_all(&files, "/a.md", "three", "3")),
        "`old_string` occurs 0 times in /a.md, and it must occur at least once. It is matched \
         exactly, so whitespace and line endings count."
    );
    assert_eq!(files, before);
}

#[test]
fn replace_all_that_is_false_or_null_needs_one_occurrence() {
    let files = Files::new().file("/a.md", "x x\n");
    for flag in [json!(false), json!(null)] {
        let args = json!({
            "path": "/a.md", "old_string": "x", "new_string": "y", "replace_all": flag
        });
        assert!(
            failure_of(replace(&files, &args)).starts_with("`old_string` occurs 2 times"),
            "{args}"
        );
    }
}

#[test]
fn the_match_is_exact_in_case_whitespace_and_line_endings() {
    let files = Files::new().file("/a.txt", "Alpha\r\n  beta\r\n");
    for miss in ["alpha", " beta\r\n ", "Alpha\n  beta"] {
        assert!(
            failure_of(edit(&files, "/a.txt", miss, "x"))
                .starts_with("`old_string` occurs 0 times"),
            "{miss:?}"
        );
    }
    assert!(edit(&files, "/a.txt", "Alpha\r\n  beta", "gamma").is_ok());
    assert_eq!(contents(&files, "/a.txt"), b"gamma\r\n");
}

#[test]
fn a_byte_order_mark_and_the_line_endings_stay_as_they_are() {
    let files = Files::new().file("/a.txt", "\u{feff}one\r\ntwo\r\n");
    assert!(edit(&files, "/a.txt", "two", "2").is_ok());
    assert_eq!(
        contents(&files, "/a.txt"),
        "\u{feff}one\r\n2\r\n".as_bytes()
    );
}

#[test]
fn the_text_put_in_is_not_searched_again() {
    let files = Files::new().file("/a.txt", "a a");
    assert_eq!(
        text_of(edit_all(&files, "/a.txt", "a", "aa")),
        "Made 2 replacements."
    );
    assert_eq!(contents(&files, "/a.txt"), b"aa aa");
}

#[test]
fn matches_are_counted_from_the_start_and_never_overlap() {
    let files = Files::new().file("/a.txt", "aaa");
    assert_eq!(
        text_of(edit(&files, "/a.txt", "aa", "b")),
        "Made 1 replacement."
    );
    assert_eq!(contents(&files, "/a.txt"), b"ba");
}

#[test]
fn a_file_that_is_not_utf8_is_refused_and_stays_as_it_is() {
    let mut utf16 = vec![0xFF, 0xFE];
    for unit in "one\n".encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    let files = Files::new()
        .file("/latin.txt", b"caf\xE9 one\n")
        .file("/utf16.txt", &utf16);
    let before = files.clone();
    for path in ["/latin.txt", "/utf16.txt"] {
        assert_eq!(
            failure_of(edit(&files, path, "one", "1")),
            format!("File is not UTF-8 text, so it cannot be edited: {path}")
        );
    }
    assert_eq!(files, before);
}

#[test]
fn a_missing_file_and_a_directory_fail_as_they_do_for_a_read() {
    let files = Files::new().file("/dir/a.md", "x\n");
    assert_eq!(
        failure_of(edit(&files, "/nope.md", "x", "y")),
        "File not found: /nope.md"
    );
    assert_eq!(
        failure_of(edit(&files, "notes/a.md", "x", "y")),
        "File not found: notes/a.md"
    );
    assert_eq!(
        failure_of(edit(&files, "/dir", "x", "y")),
        "Path is a directory, not a file: /dir"
    );
}

#[test]
fn a_pipe_is_refused_by_its_stat_and_never_read() {
    // The fake fails a read of the pipe as not found, so this text shows
    // that the edit stopped at the stat.
    let source = FakeSource::default().with("/pipe", Node::Pipe);
    assert_eq!(
        failure_of(edit(&source, "/pipe", "x", "y")),
        "Path is neither a file nor a directory: /pipe"
    );
}

#[test]
fn a_link_is_read_through_and_a_link_to_a_directory_is_a_directory() {
    let source = FakeSource::default()
        .with("/filelink", Node::LinkToFile(b"one two\n".to_vec()))
        .with("/dirlink", Node::Link);
    // The fake takes no change, so this text shows that the edit read
    // the link's file, found its one match, and asked for the write.
    assert_eq!(
        failure_of(edit(&source, "/filelink", "two", "2")),
        "the fake tree takes no change: /filelink"
    );
    assert_eq!(
        failure_of(edit(&source, "/dirlink", "two", "2")),
        "Path is a directory, not a file: /dirlink"
    );
}

#[test]
fn a_file_over_the_ceiling_is_refused_and_never_edited() {
    let files = Files::new().file("/big.txt", vec![b'a'; 16_777_217]);
    assert_eq!(
        failure_of(edit_all(&files, "/big.txt", "a", "b")),
        "File is too large to read: /big.txt is over the limit of 16777216 bytes."
    );
    assert_eq!(
        files.read("/big.txt", 0, 4),
        Ok(b"aaaa".to_vec()),
        "the file stays as it was"
    );
}

#[test]
fn an_edit_that_grows_the_file_past_one_write_is_refused_and_one_at_the_limit_is_written() {
    let mut text = String::from("X");
    text.push_str(&"a".repeat(16_777_215));
    let files = Files::new().file("/edge.txt", &text);
    assert_eq!(
        failure_of(edit(&files, "/edge.txt", "X", "XY")),
        "The edit would leave /edge.txt with 16777217 bytes, and one write takes at most \
         16777216 bytes."
    );
    assert_eq!(files.read("/edge.txt", 0, 2), Ok(b"Xa".to_vec()));
    assert!(edit(&files, "/edge.txt", "X", "Y").is_ok());
    assert_eq!(files.read("/edge.txt", 0, 2), Ok(b"Ya".to_vec()));
    assert_eq!(
        files.stat("/edge.txt"),
        Ok(Stat::new(FileType::File, 16_777_216))
    );
}

#[test]
fn an_edit_far_past_one_write_is_refused_from_the_count_before_it_is_built() {
    // Each of the 1 MiB of matches would become 16 MiB, which is 16 TiB
    // in all. No machine holds a text of that length, so a call that
    // built the edited text before it checked the length could not
    // answer.
    let files = Files::new().file("/wide.txt", "a".repeat(1_048_576));
    let wide = 1_048_576_usize.saturating_mul(16_777_216);
    assert_eq!(
        failure_of(edit_all(&files, "/wide.txt", "a", &"b".repeat(16_777_216))),
        format!(
            "The edit would leave /wide.txt with {wide} bytes, and one write takes at most \
             16777216 bytes."
        )
    );
    assert_eq!(
        files.read("/wide.txt", 0, 4),
        Ok(b"aaaa".to_vec()),
        "the file stays as it was"
    );
}

#[test]
fn the_length_of_an_edit_follows_from_the_count_of_its_matches() {
    for (text, old, new) in [
        ("x x x", "x", "yy"),
        ("x x x", "x ", ""),
        ("aaa", "aa", "b"),
        ("caf\u{e9} caf\u{e9}", "\u{e9}", "e"),
    ] {
        let count = text.matches(old).count();
        assert_eq!(
            edited_len(text.len(), count, old.len(), new.len()),
            text.replace(old, new).len(),
            "{text:?}: {old:?} to {new:?}"
        );
    }
    assert_eq!(
        edited_len(usize::MAX, usize::MAX, 1, usize::MAX),
        usize::MAX,
        "a length past the range of usize is its largest value"
    );
}

#[test]
fn a_read_the_policy_denies_fails_with_the_policys_own_reason() {
    let reason = "Mode::Plan allows only markdown paths, and /secret.rs is not one";
    let files = Files::new().fails("/secret.rs", FileError::new(FileErrorKind::Denied, reason));
    assert_eq!(failure_of(edit(&files, "/secret.rs", "x", "y")), reason);
    let source = FakeSource::default().with(
        "/late.rs",
        Node::ReadFails(FileError::new(FileErrorKind::Denied, reason)),
    );
    assert_eq!(failure_of(edit(&source, "/late.rs", "x", "y")), reason);
}

#[test]
fn a_write_the_host_fails_after_the_read_succeeded_is_worded_by_its_kind() {
    let guarded =
        |kind, text: &str| Node::Guarded(b"one two\n".to_vec(), FileError::new(kind, text));
    let reason = "Mode::Ask allows no write of /ask.md";
    let source = FakeSource::default()
        .with("/ask.md", guarded(FileErrorKind::Denied, reason))
        .with(
            "/disk.md",
            guarded(FileErrorKind::Io, "storage full: no space left"),
        );
    assert_eq!(failure_of(edit(&source, "/ask.md", "two", "2")), reason);
    assert_eq!(
        failure_of(edit(&source, "/disk.md", "two", "2")),
        "I/O failure on /disk.md: storage full: no space left"
    );
}

#[test]
fn a_missing_parameter_or_one_of_the_wrong_type_names_the_parameter() {
    let files = Files::new();
    assert_eq!(
        refused(replace(
            &files,
            &json!({ "path": "/a.md", "old_string": "x" })
        )),
        "std_StrReplace: `new_string` is required, and the call left it out"
    );
    assert_eq!(
        refused(replace(
            &files,
            &json!({
                "path": "/a.md", "old_string": "x", "new_string": "y", "replace_all": "yes"
            })
        )),
        "std_StrReplace: `replace_all` must be a JSON boolean, and the call passed \"yes\""
    );
}

#[test]
fn an_undeclared_key_is_refused_and_the_parameters_are_named() {
    let files = Files::new();
    assert_eq!(
        refused(replace(
            &files,
            &json!({ "path": "/a.md", "old_string": "x", "new_string": "y", "count": 2 })
        )),
        "std_StrReplace: `count` is not a parameter of this tool. \
         The parameters are `new_string`, `old_string`, `path` and `replace_all`."
    );
}

#[test]
fn an_empty_old_string_and_a_new_string_equal_to_it_are_refused_before_any_read() {
    // No file is there, so a call that got as far as its read would say
    // that the file was not found.
    let files = Files::new();
    assert_eq!(
        refused(edit(&files, "/a.md", "", "y")),
        "std_StrReplace: `old_string` is empty. Pass the text to replace."
    );
    assert_eq!(
        refused(edit(&files, "/a.md", "same", "same")),
        "std_StrReplace: `new_string` is the same as `old_string`, so the call would change \
         nothing"
    );
}

#[test]
fn the_refusals_come_in_a_fixed_order() {
    let files = Files::new();
    assert_eq!(
        refused(replace(
            &files,
            &json!({ "path": " ", "old_string": "", "new_string": "", "all": true })
        )),
        "std_StrReplace: `all` is not a parameter of this tool. \
         The parameters are `new_string`, `old_string`, `path` and `replace_all`."
    );
    assert_eq!(
        refused(edit(&files, " ", "", "")),
        "std_StrReplace: `path` is empty or only whitespace, and the call passed \" \""
    );
    assert_eq!(
        refused(edit(&files, "/a.md", "", "")),
        "std_StrReplace: `old_string` is empty. Pass the text to replace."
    );
}
