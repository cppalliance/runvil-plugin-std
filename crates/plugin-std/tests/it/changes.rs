//! The tools that change files, through the Plugin's entry point: which
//! tools say to the Host that they change files, what a call leaves in
//! the files of its run, what it answers, and what it asks the Host for,
//! step by step.

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
    assert_eq!(flagged, ["write", "replace", "rm"]);
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
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded.new_pack(PACK).expect("the Pack instance is made");
    let args = json!({ "path": "/a.md", "contents": "x" });
    let mut call = pack.start("write", &args.to_string());
    // The first step ends at a request, which the Host has performed
    // when the step returns: it is the write.
    assert_eq!(call.step(), Ok(None));
    assert_eq!(contents(pack.files(), "/a.md"), Ok(b"x".to_vec()));
    // The second step has the answer and ends the call.
    assert_eq!(
        call.step(),
        Ok(Some(Ok(ToolOutput::trusted("Wrote 1 byte."))))
    );
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
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(Files::new().file("/a.md", "one two\n"));
    let args = json!({ "path": "/a.md", "old_string": "two", "new_string": "2" });
    let mut call = pack.start("replace", &args.to_string());
    // The first request changes nothing, and it reads none of the bytes
    // that the edit works on: the test puts other bytes in the file once
    // the Host has performed it, and the edit is made on those.
    assert_eq!(call.step(), Ok(None));
    assert_eq!(contents(pack.files(), "/a.md"), Ok(b"one two\n".to_vec()));
    pack.files()
        .write("/a.md", "uno two\n")
        .expect("the file is written");
    // The second request is the read, which changes nothing.
    assert_eq!(call.step(), Ok(None));
    assert_eq!(contents(pack.files(), "/a.md"), Ok(b"uno two\n".to_vec()));
    // The third request is the write of the edited text.
    assert_eq!(call.step(), Ok(None));
    assert_eq!(contents(pack.files(), "/a.md"), Ok(b"uno 2\n".to_vec()));
    // The fourth step has the answer and ends the call.
    assert_eq!(
        call.step(),
        Ok(Some(Ok(ToolOutput::trusted("Made 1 replacement."))))
    );
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
fn an_edit_whose_text_overlaps_itself_fails_before_any_write_unless_it_replaces_all() {
    let before = Files::new().file("/b.txt", "banana");
    let args = json!({ "path": "/b.txt", "old_string": "ana", "new_string": "X" });
    let (result, after) = change(before.clone(), "replace", &args);
    let error = result.expect_err("the text starts at two places");
    assert_eq!(
        error.message(),
        "`old_string` starts at more than one place in /b.txt, in occurrences that overlap, and \
         it must occur exactly once. Add text around it until it names one occurrence."
    );
    assert_eq!(error.kind(), ToolErrorKind::Other);
    assert_eq!(after, before);
    assert_eq!(
        steps(before.clone(), "replace", &args),
        3,
        "the stat and the read"
    );
    let all = json!({
        "path": "/b.txt", "old_string": "ana", "new_string": "X", "replace_all": true
    });
    let (result, after) = change(before, "replace", &all);
    let output = result.expect("the occurrence that starts first is replaced");
    assert_eq!(output.text(), "Made 1 replacement.");
    assert_eq!(contents(&after, "/b.txt"), Ok(b"bXna".to_vec()));
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

#[test]
fn delete_removes_a_file_and_answers_fixed_text_as_trusted_output() {
    let (result, files) = change(
        Files::new()
            .file("/notes/a.md", "one")
            .file("/notes/b.md", "two"),
        "rm",
        &json!({ "path": "/notes/a.md" }),
    );
    let output = result.expect("the file is deleted");
    assert_eq!(output.text(), "Deleted the file.");
    assert!(output.is_trusted());
    assert_eq!(files, Files::new().file("/notes/b.md", "two"));
}

#[test]
fn delete_asks_the_host_for_a_stat_and_then_a_removal() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(Files::new().file("/a.md", "one"));
    let mut call = pack.start("rm", &json!({ "path": "/a.md" }).to_string());
    // The first request is the stat, which removes nothing.
    assert_eq!(call.step(), Ok(None));
    assert_eq!(contents(pack.files(), "/a.md"), Ok(b"one".to_vec()));
    // The second request is the removal.
    assert_eq!(call.step(), Ok(None));
    assert_eq!(pack.files(), &Files::new());
    // The third step has the answer and ends the call.
    assert_eq!(
        call.step(),
        Ok(Some(Ok(ToolOutput::trusted("Deleted the file."))))
    );
}

#[test]
fn delete_refuses_a_directory_after_its_stat_and_removes_nothing() {
    let before = Files::new().file("/notes/a.md", "one").dir("/empty");
    for directory in ["/notes", "/empty"] {
        let args = json!({ "path": directory });
        let (result, after) = change(before.clone(), "rm", &args);
        let error = result.expect_err("the path names a directory");
        assert_eq!(
            error.message(),
            format!("Path is a directory, not a file: {directory}")
        );
        assert_eq!(error.kind(), ToolErrorKind::Other);
        assert_eq!(after, before);
        assert_eq!(steps(before.clone(), "rm", &args), 2, "the stat alone");
    }
}

#[test]
fn delete_of_a_missing_path_fails_with_the_not_found_text() {
    let args = json!({ "path": "/nope.md" });
    let (result, files) = change(Files::new(), "rm", &args);
    let error = result.expect_err("nothing is at the path");
    assert_eq!(error.message(), "File not found: /nope.md");
    assert_eq!(files, Files::new());
    assert_eq!(steps(Files::new(), "rm", &args), 2, "the stat alone");
}

#[test]
fn a_delete_the_policy_denies_fails_with_the_policys_own_reason_and_changes_nothing() {
    let reason = "Mode::Ask allows no change of /vault.md";
    let before = Files::new()
        .file("/a.md", "kept")
        .fails("/vault.md", FileError::new(FileErrorKind::Denied, reason));
    let (result, after) = change(before.clone(), "rm", &json!({ "path": "/vault.md" }));
    let error = result.expect_err("the policy denies the delete");
    assert_eq!(error.message(), reason);
    assert_eq!(after, before);
}

#[test]
fn the_calls_of_one_run_write_edit_read_and_delete_one_file_in_turn() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded.new_pack(PACK).expect("the Pack instance is made");
    let call = |name: &str, args: Value| {
        pack.call(name, &args.to_string())
            .expect("the call follows the contract")
    };
    let answer = |name: &str, args: Value| {
        let output = call(name, args).unwrap_or_else(|error| panic!("{name} fails: {error}"));
        output.text().to_owned()
    };
    let path = "/notes/a.md";
    assert_eq!(
        answer("write", json!({ "path": path, "contents": "one\ntwo\n" })),
        "Wrote 8 bytes."
    );
    assert_eq!(
        answer(
            "replace",
            json!({ "path": path, "old_string": "two", "new_string": "2" })
        ),
        "Made 1 replacement."
    );
    assert_eq!(
        answer("read", json!({ "path": path })),
        "     1|one\n     2|2\n"
    );
    assert_eq!(answer("rm", json!({ "path": path })), "Deleted the file.");
    let gone = call("read", json!({ "path": path })).expect_err("the file is gone");
    assert_eq!(gone.message(), "File not found: /notes/a.md");
}
