//! The `std` Pack through the Plugin's entry point: the one Pack the
//! Plugin inserts, the seven tools an instance adds, and what a call
//! answers over the files of its run.

use plugin_std::{NOT_IMPLEMENTED, Std};
use runvil_plugin_sdk::testing::{self, Files};
use runvil_plugin_sdk::{FileError, FileErrorKind, ToolError, ToolErrorKind, ToolOutput};
use serde_json::{Value, json};

/// The name the Plugin inserts its Pack under.
pub(crate) const PACK: &str = "runvil/std";

/// Calls the tool `name` with `args` over `files`, as the Harness does:
/// load the Plugin, make one run's instance of the Pack, and run the call
/// to its end.
fn call_over(files: Files, name: &str, args: &Value) -> Result<ToolOutput, ToolError> {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(files);
    pack.call(name, &args.to_string())
        .expect("the call follows the contract")
}

/// Calls the tool `name` with `args` over a run with no files.
fn call(name: &str, args: &Value) -> Result<ToolOutput, ToolError> {
    call_over(Files::new(), name, args)
}

#[test]
fn the_plugin_inserts_the_one_pack_runvil_std() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let packs: Vec<(&str, &str)> = loaded
        .packs()
        .iter()
        .map(|pack| (pack.name.as_str(), pack.description.as_str()))
        .collect();
    assert_eq!(packs, [(PACK, "Read, write, search, and delete files")]);
}

#[test]
fn an_instance_adds_seven_tools_each_under_its_wire_name() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded.new_pack(PACK).expect("the Pack instance is made");
    let names: Vec<(&str, Option<&str>)> = pack
        .tools()
        .iter()
        .map(|tool| (tool.name.as_str(), tool.wire_name.as_deref()))
        .collect();
    assert_eq!(
        names,
        [
            ("read", Some("Read")),
            ("write", Some("Write")),
            ("replace", Some("StrReplace")),
            ("rm", Some("Delete")),
            ("glob", Some("Glob")),
            ("grep", Some("Grep")),
            ("dir", Some("ListDir")),
        ]
    );
}

#[test]
fn each_run_gets_an_instance_of_its_own_with_the_same_tools() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let first = loaded.new_pack(PACK).expect("the Pack instance is made");
    let second = loaded.new_pack(PACK).expect("the Pack instance is made");
    assert_eq!(first.tools(), second.tools());
}

#[test]
fn every_stub_answers_the_not_implemented_text_naming_itself() {
    for (name, wire_name, args) in [
        (
            "write",
            "Write",
            json!({ "path": "/notes.md", "contents": "hi" }),
        ),
        (
            "replace",
            "StrReplace",
            json!({ "path": "/notes.md", "old_string": "a", "new_string": "b" }),
        ),
        ("rm", "Delete", json!({ "path": "/notes.md" })),
        ("glob", "Glob", json!({ "glob_pattern": "**/*.md" })),
    ] {
        let output = call(name, &args).expect("a matching call succeeds");
        let expected = NOT_IMPLEMENTED.replace("<name>", wire_name);
        assert_eq!(output.text(), expected, "{name}");
        assert!(output.is_trusted(), "{name}");
    }
}

#[test]
fn the_not_implemented_text_says_the_result_never_changes() {
    let text = NOT_IMPLEMENTED.replace("<name>", "Read");
    assert!(
        text.contains("nothing in this session will change that"),
        "{text}"
    );
    assert!(text.contains("Calling Read again is safe"), "{text}");
    assert!(text.contains("this same result every time"), "{text}");
    assert!(!text.contains('<'), "the name is filled in: {text}");
}

#[test]
fn a_missing_required_parameter_fails_naming_it() {
    let error = call("grep", &json!({})).expect_err("pattern is required");
    assert_eq!(
        error.message(),
        "std_Grep: `pattern` is required, and the call left it out"
    );
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
}

#[test]
fn a_parameter_of_the_wrong_type_fails_naming_it() {
    let error = call("read", &json!({ "path": 7 })).expect_err("path is a string");
    assert_eq!(
        error.message(),
        "std_Read: `path` must be a JSON string, and the call passed 7"
    );
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
}

#[test]
fn a_value_outside_an_enum_fails_listing_the_allowed_values() {
    let error = call("grep", &json!({ "pattern": "x", "output_mode": "lines" }))
        .expect_err("output_mode has three allowed values");
    assert_eq!(
        error.message(),
        "std_Grep: `output_mode` must be one of \"content\", \"files_with_matches\", \"count\", \
         and the call passed \"lines\""
    );
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
}

#[test]
fn arguments_that_are_not_an_object_fail_as_invalid_arguments() {
    let error = call("read", &json!(["/notes.md"])).expect_err("the arguments are an array");
    assert_eq!(
        error.message(),
        "std_Read: the arguments must be a JSON object"
    );
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
}

#[test]
fn read_answers_the_numbered_lines_of_a_file_as_untrusted_output() {
    let output = call_over(
        Files::new().file("/notes.md", "hello\nworld\n"),
        "read",
        &json!({ "path": "/notes.md" }),
    )
    .expect("the file reads");
    assert_eq!(output.text(), "     1|hello\n     2|world\n");
    assert!(!output.is_trusted());
}

#[test]
fn read_answers_file_is_empty_for_a_zero_byte_file() {
    let output = call_over(
        Files::new().file("/empty.md", ""),
        "read",
        &json!({ "path": "/empty.md" }),
    )
    .expect("an empty file reads");
    assert_eq!(output.text(), "File is empty.");
    assert!(!output.is_trusted());
}

#[test]
fn read_of_a_missing_path_fails_with_the_not_found_text() {
    let error = call("read", &json!({ "path": "/nope.md" })).expect_err("the file is missing");
    assert_eq!(error.message(), "File not found: /nope.md");
    assert_eq!(error.kind(), ToolErrorKind::Other);
}

#[test]
fn a_read_the_policy_denies_fails_with_the_policys_own_reason() {
    let reason = "Mode::Ask allows no read of /vault";
    let files = Files::new().fails("/vault", FileError::new(FileErrorKind::Denied, reason));
    for (name, args) in [
        ("read", json!({ "path": "/vault" })),
        ("dir", json!({ "target_directory": "/vault" })),
        ("grep", json!({ "pattern": "x", "path": "/vault" })),
    ] {
        let error = call_over(files.clone(), name, &args).expect_err("the policy denies it");
        assert_eq!(error.message(), reason, "{name}");
    }
}

#[test]
fn list_dir_answers_the_tree_of_a_directory_as_untrusted_output() {
    let files = Files::new()
        .file("/proj/README.md", "hello")
        .file("/proj/src/lib.rs", "fn main() {}")
        .file("/proj/.git/HEAD", "ref");
    let output = call_over(files, "dir", &json!({ "target_directory": "/proj" }))
        .expect("the directory lists");
    assert_eq!(
        output.text(),
        "/proj/\n  - README.md\n  - src/\n    - lib.rs"
    );
    assert!(!output.is_trusted());
}

#[test]
fn list_dir_of_a_missing_path_fails_with_the_does_not_exist_text() {
    let error =
        call("dir", &json!({ "target_directory": "/nope" })).expect_err("the directory is missing");
    assert_eq!(error.message(), "Path does not exist: /nope");
}

#[test]
fn list_dir_leaves_out_what_ignore_globs_match() {
    let files = Files::new()
        .file("/proj/a.md", "x")
        .file("/proj/b.log", "y");
    let output = call_over(
        files,
        "dir",
        &json!({ "target_directory": "/proj", "ignore_globs": ["*.log"] }),
    )
    .expect("the directory lists");
    assert_eq!(output.text(), "/proj/\n  - a.md");
}

#[test]
fn grep_answers_the_matching_lines_of_a_tree_as_untrusted_output() {
    let files = Files::new()
        .file(
            "/src/a.ts",
            "// one\n// two\nconst alpha = 1\nlet beta = 2\n// five\n// six\n// seven\n\
             // eight\nconst gamma = 3\nexport default gamma\n",
        )
        .file("/src/b.ts", "const first = 0\n");
    let output = call_over(files, "grep", &json!({ "pattern": "const", "-A": 1 }))
        .expect("the tree searches");
    assert_eq!(
        output.text(),
        "/src/a.ts\n  3:const alpha = 1\n  4-let beta = 2\n  --\n  9:const gamma = 3\n  \
         10-export default gamma\n\n/src/b.ts\n  1:const first = 0"
    );
    assert!(!output.is_trusted());
}

#[test]
fn grep_of_a_missing_path_fails_with_the_does_not_exist_text() {
    let error =
        call("grep", &json!({ "pattern": "x", "path": "/nope" })).expect_err("the path is missing");
    assert_eq!(error.message(), "Path does not exist: /nope");
}

#[test]
fn grep_refuses_an_undeclared_key_by_name_while_the_stubs_let_it_through() {
    let error = call("grep", &json!({ "pattern": "x", "context": 3 }))
        .expect_err("context is not a parameter");
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
    assert!(
        error
            .message()
            .starts_with("std_Grep: `context` is not a parameter of this tool."),
        "{error}"
    );
    let stub = call("glob", &json!({ "glob_pattern": "*.md", "extra": 1 }))
        .expect("a stub ignores an undeclared key");
    assert_eq!(stub.text(), NOT_IMPLEMENTED.replace("<name>", "Glob"));
}
