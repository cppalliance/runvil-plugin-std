//! The tools that change files, through the Plugin's entry point: which
//! tools say to the Host that they change files, what a call leaves in
//! the files of its run, what it answers, and how many requests it makes.

use plugin_std::Std;
use runvil_plugin_sdk::testing::{self, Files};
use runvil_plugin_sdk::{FileError, FileErrorKind, ToolError, ToolErrorKind, ToolOutput};
use serde_json::{Value, json};

use crate::tools::PACK;

/// Calls the tool `name` with `args` over `files`, and returns what the
/// call answered beside the files as the call left them.
fn change(files: Files, name: &str, args: &Value) -> (Result<ToolOutput, ToolError>, Files) {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(files);
    let result = pack
        .call(name, &args.to_string())
        .expect("the call follows the contract");
    (result, pack.files().clone())
}

/// How many steps a call of the tool `name` with `args` over `files`
/// takes to its end. Each file request ends one step, so a call that
/// makes `n` requests takes `n + 1` steps.
fn steps(files: Files, name: &str, args: &Value) -> usize {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(files);
    let mut call = pack.start(name, &args.to_string());
    let mut steps = 1;
    while call
        .step()
        .expect("each step follows the contract")
        .is_none()
    {
        steps += 1;
    }
    steps
}

/// The whole contents of the file at `path` in `files`.
fn contents(files: &Files, path: &str) -> Result<Vec<u8>, FileError> {
    files.read(path, 0, u64::MAX)
}

#[test]
fn the_tools_that_change_files_say_so_to_the_host_and_no_other_tool_does() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded.new_pack(PACK).expect("the Pack instance is made");
    let flagged: Vec<&str> = pack
        .tools()
        .iter()
        .filter(|tool| tool.changes_files)
        .map(|tool| tool.name.as_str())
        .collect();
    assert_eq!(flagged, ["write", "replace"]);
}

#[test]
fn write_makes_a_file_and_answers_the_byte_count_as_trusted_output() {
    let (result, files) = change(
        Files::new(),
        "write",
        &json!({ "path": "/notes/a.md", "contents": "hello\n" }),
    );
    let output = result.expect("the file is written");
    assert_eq!(output.text(), "Wrote 6 bytes.");
    assert!(output.is_trusted());
    assert_eq!(contents(&files, "/notes/a.md"), Ok(b"hello\n".to_vec()));
}

#[test]
fn write_replaces_the_whole_contents_of_a_file() {
    let (result, files) = change(
        Files::new().file("/a.md", "a long first version\n"),
        "write",
        &json!({ "path": "/a.md", "contents": "short" }),
    );
    assert!(result.is_ok());
    assert_eq!(contents(&files, "/a.md"), Ok(b"short".to_vec()));
}

#[test]
fn write_asks_the_host_for_one_write_and_nothing_else() {
    let args = json!({ "path": "/a.md", "contents": "x" });
    assert_eq!(steps(Files::new(), "write", &args), 2);
}

#[test]
fn a_write_the_policy_denies_fails_with_the_policys_own_reason_and_changes_nothing() {
    let reason = "Mode::Ask allows no write of /vault.md";
    let before = Files::new()
        .file("/a.md", "kept")
        .fails("/vault.md", FileError::new(FileErrorKind::Denied, reason));
    let (result, after) = change(
        before.clone(),
        "write",
        &json!({ "path": "/vault.md", "contents": "x" }),
    );
    let error = result.expect_err("the policy denies the write");
    assert_eq!(error.message(), reason);
    assert_eq!(error.kind(), ToolErrorKind::Other);
    assert_eq!(after, before);
}

#[test]
fn contents_over_the_limit_of_one_write_fail_naming_the_limit_before_the_host_is_asked() {
    let args = json!({ "path": "/big.txt", "contents": "a".repeat(16_777_217) });
    let (result, files) = change(Files::new(), "write", &args);
    let error = result.expect_err("the contents are over the limit");
    assert_eq!(
        error.message(),
        "std_Write: `contents` is 16777217 bytes, and one write takes at most 16777216 bytes"
    );
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
    assert_eq!(files, Files::new());
    assert_eq!(steps(Files::new(), "write", &args), 1);
}

#[test]
fn a_path_longer_than_the_contract_takes_fails_before_the_host_is_asked() {
    let args = json!({ "path": format!("/{}", "a".repeat(4096)), "contents": "x" });
    let (result, files) = change(Files::new(), "write", &args);
    let error = result.expect_err("the path is over the limit");
    assert_eq!(error.message(), "the path is longer than 4096 bytes");
    assert_eq!(files, Files::new());
    assert_eq!(steps(Files::new(), "write", &args), 1);
}

#[test]
fn a_later_call_of_the_run_reads_what_a_write_left() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded.new_pack(PACK).expect("the Pack instance is made");
    let written = pack
        .call(
            "write",
            &json!({ "path": "/notes/a.md", "contents": "one\ntwo\n" }).to_string(),
        )
        .expect("the call follows the contract");
    assert!(written.is_ok());
    let read = pack
        .call("read", r#"{"path":"/notes/a.md"}"#)
        .expect("the call follows the contract")
        .expect("the file reads");
    assert_eq!(read.text(), "     1|one\n     2|two\n");
}

#[test]
fn str_replace_edits_the_one_occurrence_and_answers_the_count_as_trusted_output() {
    let (result, files) = change(
        Files::new().file("/a.md", "one\ntwo\nthree\n"),
        "replace",
        &json!({ "path": "/a.md", "old_string": "two", "new_string": "2" }),
    );
    let output = result.expect("the file is edited");
    assert_eq!(output.text(), "Made 1 replacement.");
    assert!(output.is_trusted());
    assert_eq!(contents(&files, "/a.md"), Ok(b"one\n2\nthree\n".to_vec()));
}

#[test]
fn str_replace_with_replace_all_edits_every_occurrence() {
    let (result, files) = change(
        Files::new().file("/a.rs", "let x = x + x;\n"),
        "replace",
        &json!({
            "path": "/a.rs", "old_string": "x", "new_string": "total", "replace_all": true
        }),
    );
    let output = result.expect("the file is edited");
    assert_eq!(output.text(), "Made 3 replacements.");
    assert_eq!(
        contents(&files, "/a.rs"),
        Ok(b"let total = total + total;\n".to_vec())
    );
}

#[test]
fn str_replace_asks_the_host_for_a_stat_a_read_and_a_write() {
    let files = Files::new().file("/a.md", "one two\n");
    let args = json!({ "path": "/a.md", "old_string": "two", "new_string": "2" });
    assert_eq!(steps(files, "replace", &args), 4);
}

#[test]
fn an_edit_whose_text_does_not_occur_once_fails_with_the_count_before_any_write() {
    let before = Files::new().file("/a.rs", "let x = x + x;\n");
    let args = json!({ "path": "/a.rs", "old_string": "x", "new_string": "total" });
    let (result, after) = change(before.clone(), "replace", &args);
    let error = result.expect_err("the text occurs three times");
    assert_eq!(
        error.message(),
        "`old_string` occurs 3 times in /a.rs, and it must occur exactly once. Add text around \
         it until it names one occurrence, or set `replace_all` to replace every one."
    );
    assert_eq!(error.kind(), ToolErrorKind::Other);
    assert_eq!(after, before);
    assert_eq!(steps(before, "replace", &args), 3, "the stat and the read");
}

#[test]
fn an_edit_the_policy_denies_fails_with_the_policys_own_reason_and_changes_nothing() {
    let reason = "Mode::Ask allows no read of /vault.md";
    let before = Files::new()
        .file("/a.md", "kept")
        .fails("/vault.md", FileError::new(FileErrorKind::Denied, reason));
    let (result, after) = change(
        before.clone(),
        "replace",
        &json!({ "path": "/vault.md", "old_string": "a", "new_string": "b" }),
    );
    let error = result.expect_err("the policy denies the edit");
    assert_eq!(error.message(), reason);
    assert_eq!(after, before);
}

#[test]
fn two_edits_of_one_file_in_a_run_both_apply() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(Files::new().file("/a.md", "one two three\n"));
    for (old, new) in [("one", "1"), ("three", "3")] {
        let args = json!({ "path": "/a.md", "old_string": old, "new_string": new });
        let edited = pack
            .call("replace", &args.to_string())
            .expect("the call follows the contract");
        assert!(edited.is_ok(), "{old}");
    }
    assert_eq!(contents(pack.files(), "/a.md"), Ok(b"1 two 3\n".to_vec()));
}
