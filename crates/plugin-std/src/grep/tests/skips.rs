//! What Grep skips and counts instead of failing, and how the limits and
//! the pauses behave, through a [`Source`](crate::source::Source) that
//! can produce what the fake Host's tree cannot.

use runvil_plugin_sdk::testing::Files;
use runvil_plugin_sdk::{FileError, FileErrorKind};
use serde_json::json;

use super::{block_on, count_pendings, definition, failure_of, grep_over, text_of};
use crate::fake::{FakeSource, Node};
use crate::grep::args;
use crate::grep::found::Limits;

/// The error for a path the policy refuses.
fn denied() -> FileError {
    FileError::new(FileErrorKind::Denied, "the policy refuses it")
}

/// The error for a path another call of the round holds.
fn conflict() -> FileError {
    FileError::new(FileErrorKind::Conflict, "conflicting claim: another call")
}

/// The error for a path the operating system failed.
fn io_failure() -> FileError {
    FileError::new(FileErrorKind::Io, "other error: disk error")
}

/// A tree with one of everything Grep skips, and one clean match.
fn messy() -> FakeSource {
    FakeSource::default()
        .with("/a.bin", Node::File(b"NEEDLE\0\0".to_vec()))
        .file("/bad\u{1}name", "NEEDLE\n")
        .with("/big.log", Node::Sized(1000, b"NEEDLE\n".to_vec()))
        .with("/busy.txt", Node::ReadFails(conflict()))
        .with("/denied.txt", Node::ReadFails(denied()))
        .with("/deny", Node::ListFails(denied()))
        .with("/io.txt", Node::ReadFails(io_failure()))
        .with("/link", Node::Link)
        .with("/pipe", Node::Pipe)
        .file("/ok.txt", "NEEDLE here\n")
}

/// The limits the skip tests lower the file size under.
fn small_files() -> Limits {
    Limits {
        max_file_size: 100,
        ..Limits::default()
    }
}

/// The skip line every `messy` search ends with.
const MESSY_SKIPS: &str = "[Skipped 9 entries: 1 binary file, 2 files that could not be read, 1 \
     file over 100 bytes, 1 symbolic link, 1 name with a control character, 1 entry held by \
     another call in this round, 2 entries denied by policy. First: /a.bin, /big.log, \
     /busy.txt, /denied.txt, /deny]";

#[test]
fn every_kind_of_skip_is_counted_and_the_clean_match_is_still_found() {
    let output = text_of(grep_over(
        &messy(),
        &json!({ "pattern": "NEEDLE" }),
        &small_files(),
    ));
    assert_eq!(output, format!("/ok.txt\n  1:NEEDLE here\n\n{MESSY_SKIPS}"));
}

#[test]
fn a_zero_match_result_keeps_the_skip_line_and_the_searched_count() {
    let output = grep_over(&messy(), &json!({ "pattern": "absent" }), &small_files())
        .expect("the search succeeds");
    assert!(!output.is_trusted(), "it names skipped paths");
    assert_eq!(
        output.text(),
        format!(
            "No matches found. Searched 1 file.\n\n{MESSY_SKIPS}\n\
             [No ignore rules ran: .gitignore is not applied, and only entries named .git are \
             skipped.]"
        )
    );
}

#[test]
fn a_binary_file_is_skipped_in_every_output_mode() {
    let source = FakeSource::default().with("/a.bin", Node::File(b"NEEDLE\0".to_vec()));
    for mode in ["content", "files_with_matches", "count"] {
        let output = grep_over(
            &source,
            &json!({ "pattern": "NEEDLE", "output_mode": mode }),
            &Limits::default(),
        )
        .expect("the search succeeds");
        assert_eq!(
            output.text(),
            "No matches found. Searched 0 files.\n\n\
             [Skipped 1 entry: 1 binary file. First: /a.bin]\n\
             [No ignore rules ran: .gitignore is not applied, and only entries named .git are \
             skipped.]",
            "{mode}"
        );
    }
}

#[test]
fn the_skip_line_for_one_skip_uses_the_singular() {
    let source = FakeSource::default()
        .with("/a.bin", Node::File(b"x\0".to_vec()))
        .file("/ok.txt", "NEEDLE\n");
    let output = text_of(grep_over(
        &source,
        &json!({ "pattern": "NEEDLE" }),
        &Limits::default(),
    ));
    assert_eq!(
        output,
        "/ok.txt\n  1:NEEDLE\n\n[Skipped 1 entry: 1 binary file. First: /a.bin]"
    );
}

#[test]
fn a_control_character_name_is_counted_and_never_printed() {
    let source = FakeSource::default()
        .file("/a\u{7f}b", "NEEDLE\n")
        .file("/ok.txt", "NEEDLE\n");
    let output = text_of(grep_over(
        &source,
        &json!({ "pattern": "NEEDLE" }),
        &Limits::default(),
    ));
    assert_eq!(
        output,
        "/ok.txt\n  1:NEEDLE\n\n[Skipped 1 entry: 1 name with a control character.]"
    );
}

#[test]
fn only_the_first_five_printable_skips_are_named() {
    let mut source = FakeSource::default();
    for index in 0..8 {
        source = source.with(&format!("/f{index}.bin"), Node::File(b"\0".to_vec()));
    }
    let output = grep_over(&source, &json!({ "pattern": "x" }), &Limits::default())
        .expect("the search succeeds");
    assert!(
        output.text().contains(
            "[Skipped 8 entries: 8 binary files. First: /f0.bin, /f1.bin, /f2.bin, /f3.bin, \
             /f4.bin]"
        ),
        "{}",
        output.text()
    );
}

#[test]
fn a_directory_that_cannot_be_listed_is_skipped_by_why() {
    let source = FakeSource::default()
        .with("/io", Node::ListFails(io_failure()))
        .with("/locked", Node::ListFails(conflict()))
        .with("/policy", Node::ListFails(denied()))
        .file("/ok.txt", "NEEDLE\n");
    let output = text_of(grep_over(
        &source,
        &json!({ "pattern": "NEEDLE" }),
        &Limits::default(),
    ));
    assert_eq!(
        output,
        "/ok.txt\n  1:NEEDLE\n\n[Skipped 3 entries: 1 directory that could not be read, 1 entry \
         held by another call in this round, 1 entry denied by policy. First: /io, /locked, \
         /policy]"
    );
}

#[test]
fn a_pipe_is_never_opened_and_is_skipped_as_unreadable() {
    let source = FakeSource::default().with("/pipe", Node::Pipe);
    let output = grep_over(&source, &json!({ "pattern": "x" }), &Limits::default())
        .expect("the search succeeds");
    assert!(
        output
            .text()
            .contains("[Skipped 1 entry: 1 file that could not be read. First: /pipe]"),
        "{}",
        output.text()
    );
}

#[test]
fn a_file_over_the_size_limit_is_skipped_before_any_read() {
    let source = FakeSource::default().with("/big.log", Node::Sized(1000, b"NEEDLE\n".to_vec()));
    let output = grep_over(&source, &json!({ "pattern": "NEEDLE" }), &small_files())
        .expect("the search succeeds");
    assert!(
        output
            .text()
            .contains("[Skipped 1 entry: 1 file over 100 bytes. First: /big.log]"),
        "{}",
        output.text()
    );
}

#[test]
fn a_missing_path_fails_with_the_does_not_exist_text() {
    let source = FakeSource::default().file("/a.txt", "x\n");
    let message = failure_of(grep_over(
        &source,
        &json!({ "pattern": "x", "path": "/x" }),
        &Limits::default(),
    ));
    assert_eq!(message, "Path does not exist: /x");
}

#[test]
fn a_root_that_cannot_be_listed_or_statted_fails_through_the_refusal_text() {
    let source = FakeSource::default().with("/deny", Node::ListFails(denied()));
    let message = failure_of(grep_over(
        &source,
        &json!({ "pattern": "x", "path": "/deny" }),
        &Limits::default(),
    ));
    assert_eq!(message, "the policy refuses it");
}

#[test]
fn a_root_that_is_a_pipe_is_neither_a_file_nor_a_directory() {
    let source = FakeSource::default().with("/pipe", Node::Pipe);
    let message = failure_of(grep_over(
        &source,
        &json!({ "pattern": "x", "path": "/pipe" }),
        &Limits::default(),
    ));
    assert_eq!(message, "Path is neither a file nor a directory: /pipe");
}

#[test]
fn a_named_file_is_searched_alone_and_a_failed_read_of_it_fails_the_call() {
    let source = FakeSource::default()
        .file("/a.txt", "NEEDLE\n")
        .with("/denied.txt", Node::ReadFails(denied()));
    let output = text_of(grep_over(
        &source,
        &json!({ "pattern": "NEEDLE", "path": "/a.txt" }),
        &Limits::default(),
    ));
    assert_eq!(output, "/a.txt\n  1:NEEDLE");
    let message = failure_of(grep_over(
        &source,
        &json!({ "pattern": "NEEDLE", "path": "/denied.txt" }),
        &Limits::default(),
    ));
    assert_eq!(message, "the policy refuses it");
}

#[test]
fn a_named_file_over_the_size_limit_is_a_skip() {
    let source = FakeSource::default().with("/big", Node::Sized(1000, b"NEEDLE\n".to_vec()));
    let output = grep_over(
        &source,
        &json!({ "pattern": "NEEDLE", "path": "/big" }),
        &small_files(),
    )
    .expect("the search succeeds");
    assert!(
        output
            .text()
            .starts_with("No matches found. Searched 0 files.\n\n[Skipped 1 entry: 1 file over 100 bytes. First: /big]"),
        "{}",
        output.text()
    );
}

#[test]
fn a_named_link_is_searched_through_as_a_directory_and_then_as_a_file() {
    let source = FakeSource::default()
        .with("/dirlink", Node::Link)
        .file("/dirlink/a.txt", "NEEDLE\n")
        .with("/filelink", Node::LinkToFile(b"NEEDLE inside\n".to_vec()));
    let through_dir = text_of(grep_over(
        &source,
        &json!({ "pattern": "NEEDLE", "path": "/dirlink" }),
        &Limits::default(),
    ));
    assert_eq!(through_dir, "/dirlink/a.txt\n  1:NEEDLE");
    let through_file = text_of(grep_over(
        &source,
        &json!({ "pattern": "NEEDLE", "path": "/filelink" }),
        &Limits::default(),
    ));
    assert_eq!(through_file, "/filelink\n  1:NEEDLE inside");
}

#[test]
fn a_named_link_that_cannot_be_listed_fails_the_call_in_the_words_for_its_kind() {
    let source = FakeSource::default().with("/link", Node::LinkFails(denied()));
    let message = failure_of(grep_over(
        &source,
        &json!({ "pattern": "x", "path": "/link" }),
        &Limits::default(),
    ));
    assert_eq!(message, "the policy refuses it");
}

#[test]
fn a_named_file_without_a_leading_slash_is_printed_from_the_root() {
    let files = Files::new().file("/notes/a.md", "NEEDLE\n");
    let output = text_of(grep_over(
        &files,
        &json!({ "pattern": "NEEDLE", "path": "notes/a.md" }),
        &Limits::default(),
    ));
    assert_eq!(output, "/notes/a.md\n  1:NEEDLE");
}

#[test]
fn a_named_link_to_an_over_size_file_is_a_skip_found_after_a_bounded_read() {
    let source = FakeSource::default().with("/filelink", Node::LinkToFile(vec![b'a'; 500]));
    let output = grep_over(
        &source,
        &json!({ "pattern": "a", "path": "/filelink" }),
        &small_files(),
    )
    .expect("the search succeeds");
    assert!(
        output
            .text()
            .contains("[Skipped 1 entry: 1 file over 100 bytes. First: /filelink]"),
        "{}",
        output.text()
    );
}

#[test]
fn the_search_pauses_once_per_slice_of_files() {
    let mut source = FakeSource::default();
    for index in 0..10 {
        source = source.file(&format!("/f{index}"), "x\n");
    }
    let limits = Limits {
        slice_entries: 3,
        ..Limits::default()
    };
    let definition = definition();
    let args = json!({ "pattern": "x", "output_mode": "count" });
    let request = args::parse(definition, &args).expect("the arguments parse");
    let (output, pending) = count_pendings(crate::grep::search(&source, "Grep", &request, &limits));
    assert!(output.is_ok());
    assert_eq!(pending, 3, "after files 3, 6, and 9");
}

#[test]
fn a_tree_of_directories_with_no_files_still_pauses_once_per_slice() {
    let source = (0..7).fold(Files::new(), |tree, index| {
        tree.dir(&format!("/d{index}/inner"))
    });
    let limits = Limits {
        slice_entries: 4,
        ..Limits::default()
    };
    let definition = definition();
    let args = json!({ "pattern": "x" });
    let request = args::parse(definition, &args).expect("the arguments parse");
    let (output, pending) = count_pendings(crate::grep::search(&source, "Grep", &request, &limits));
    assert_eq!(
        output.expect("the search succeeds").text(),
        "No matches found. Searched 0 files.\n\n\
         [No ignore rules ran: .gitignore is not applied, and only entries named .git are \
         skipped.]"
    );
    assert_eq!(pending, 3, "after directories 4, 8, and 12 of the 14");
}

#[test]
fn the_search_pauses_once_per_slice_of_bytes() {
    let mut source = FakeSource::default();
    for index in 0..10 {
        source = source.file(&format!("/f{index}"), "0123456789");
    }
    let limits = Limits {
        slice_bytes: 25,
        ..Limits::default()
    };
    let definition = definition();
    let args = json!({ "pattern": "x" });
    let request = args::parse(definition, &args).expect("the arguments parse");
    let (output, pending) = count_pendings(crate::grep::search(&source, "Grep", &request, &limits));
    assert!(output.is_ok());
    assert_eq!(pending, 3, "after files 3, 6, and 9, at 30 bytes each time");
}

#[test]
fn a_search_dropped_after_its_first_pause_leaves_the_source_usable() {
    let mut source = FakeSource::default();
    for index in 0..10 {
        source = source.file(&format!("/f{index}"), "x\n");
    }
    let limits = Limits {
        slice_entries: 2,
        ..Limits::default()
    };
    let definition = definition();
    let args = json!({ "pattern": "x" });
    let request = args::parse(definition, &args).expect("the arguments parse");
    {
        let mut future = Box::pin(crate::grep::search(&source, "Grep", &request, &limits));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(future.as_mut().poll(&mut cx).is_pending());
    }
    let again = block_on(crate::grep::search(&source, "Grep", &request, &limits));
    assert!(again.is_ok(), "a fresh search still runs");
}
