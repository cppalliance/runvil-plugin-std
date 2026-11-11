//! Unit tests for the `std` Pack's stub text, the definitions it builds
//! from, the arguments it parses, and the tool a call's name picks.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{ToolError, ToolErrorKind, ToolOutput};

use super::{INFO, NOT_IMPLEMENTED, StdPack, parse, run};
use crate::spec;

/// Runs one call of the tool `name` with `arguments` over `files` to its
/// end, polling again after each `Pending`.
fn ran(name: &str, arguments: &str, files: &Files) -> Result<ToolOutput, ToolError> {
    let mut future = pin!(run(name, arguments, files));
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(result) = future.as_mut().poll(&mut cx) {
            return result;
        }
    }
}

#[test]
fn the_pack_is_runvil_std_with_its_one_line_description() {
    assert_eq!(INFO.name, "runvil/std");
    assert_eq!(INFO.description, "Read, write, search, and delete files");
}

#[test]
fn the_stub_names_the_tool_in_every_place_the_text_mentions_it() {
    let text = NOT_IMPLEMENTED.replace("<name>", "Grep");
    assert_eq!(
        text.matches("Grep").count(),
        2,
        "the text names the tool twice: {text}"
    );
    assert!(!text.contains("<name>"), "{text}");
}

#[test]
fn the_stub_says_the_implementation_is_missing_and_the_result_is_stable() {
    let text = NOT_IMPLEMENTED.replace("<name>", "Write");
    for phrase in [
        "not implemented in this build",
        "The implementation is missing",
        "nothing in this session will change that",
        "Calling Write again is safe",
        "this same result every time",
        "Carry on without it.",
    ] {
        assert!(text.contains(phrase), "missing {phrase:?}: {text}");
    }
}

#[test]
fn the_stub_says_what_is_missing_before_it_says_anything_else() {
    let text = NOT_IMPLEMENTED.replace("<name>", "Read");
    assert!(
        text.starts_with("The Read tool is not implemented"),
        "the first clause says what is missing: {text}"
    );
}

#[test]
fn the_specs_are_the_seven_the_crate_names_in_order() {
    let segments: Vec<&str> = spec::all()
        .expect("every embedded definition parses")
        .iter()
        .map(|definition| definition.segment)
        .collect();
    assert_eq!(
        segments,
        ["read", "write", "replace", "rm", "glob", "grep", "dir"]
    );
}

#[test]
fn an_instance_names_each_tool_by_its_segment_with_its_schema_as_json_text() {
    let pack = StdPack::new().expect("every embedded definition parses");
    let definitions = spec::all().expect("every embedded definition parses");
    assert_eq!(pack.tools.len(), definitions.len());
    for (tool, definition) in pack.tools.iter().zip(definitions) {
        assert_eq!(tool.name, definition.segment);
        assert_eq!(
            tool.wire_name.as_deref(),
            Some(definition.wire_name.as_str())
        );
        assert_eq!(tool.description, definition.description);
        let parameters: serde_json::Value =
            serde_json::from_str(&tool.parameters).expect("the schema is JSON text");
        assert_eq!(parameters, definition.parameters, "{}", tool.name);
    }
}

#[test]
fn arguments_that_are_not_json_fail_as_invalid_arguments_naming_the_tool() {
    assert_eq!(
        parse("Read", r#"{"path":"/a.md"}"#),
        Ok(serde_json::json!({ "path": "/a.md" }))
    );
    let error = parse("Read", "{").expect_err("the text is not JSON");
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
    assert!(
        error
            .message()
            .starts_with("std_Read: the arguments are not JSON: "),
        "{error}"
    );
}

#[test]
fn a_call_of_a_tool_the_pack_lacks_fails_naming_it() {
    let error = ran("copy", "{}", &Files::new()).expect_err("std has no copy tool");
    assert_eq!(error.kind(), ToolErrorKind::Other);
    assert_eq!(error.message(), "std has no tool named copy");
}

#[test]
fn a_call_checks_its_arguments_against_the_schema_before_the_tool_runs() {
    let files = Files::new().file("/a.md", "one\n");
    let error = ran("read", r#"{"path":7}"#, &files).expect_err("path is a string");
    assert_eq!(error.kind(), ToolErrorKind::InvalidArguments);
    assert_eq!(
        error.message(),
        "std_Read: `path` must be a JSON string, and the call passed 7"
    );
    let not_json = ran("read", "path=/a.md", &files).expect_err("the text is not JSON");
    assert_eq!(not_json.kind(), ToolErrorKind::InvalidArguments);
}

#[test]
fn a_call_goes_to_the_tool_its_name_picks() {
    let files = Files::new()
        .file("/p/a.md", "needle\n")
        .file("/p/b.md", "hay\n");
    let read = ran("read", r#"{"path":"/p/a.md"}"#, &files).expect("the file reads");
    assert_eq!(read.text(), "     1|needle\n");
    let dir = ran("dir", r#"{"target_directory":"/p"}"#, &files).expect("the directory lists");
    assert_eq!(dir.text(), "/p/\n  - a.md\n  - b.md");
    let grep = ran("grep", r#"{"pattern":"needle"}"#, &files).expect("the tree searches");
    assert_eq!(grep.text(), "/p/a.md\n  1:needle");
    let stub = ran("write", r#"{"path":"/p/c.md","contents":"x"}"#, &files)
        .expect("a stub answers its text");
    assert_eq!(stub.text(), NOT_IMPLEMENTED.replace("<name>", "Write"));
    assert!(stub.is_trusted());
}
