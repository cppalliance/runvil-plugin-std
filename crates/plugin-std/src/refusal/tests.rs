//! Unit tests for the refusal texts: one sentence per kind of
//! `FileError`, byte for byte, with the path echoed as the model wrote it.

use runvil_plugin_sdk::{FileError, FileErrorKind};

use super::{missing_root, neither_file_nor_directory, not_a_directory, refusal};

/// The path the model wrote, and the resolved path every error text
/// below names, which differs so a test can tell which one a sentence
/// echoes.
const WRITTEN: &str = "notes/a.md";
const RESOLVED: &str = "/resolved/a.md";

/// A failure of `kind` whose text is the Host's sentence about the
/// resolved path.
fn error(kind: FileErrorKind) -> FileError {
    FileError::new(kind, format!("the Host's words about {RESOLVED}"))
}

/// What `refusal` says for `error` when `Read` was called on
/// `notes/a.md`.
fn say(error: &FileError) -> String {
    refusal("Read", WRITTEN, error)
}

#[test]
fn not_found_says_the_file_was_not_found_with_the_path_as_written() {
    assert_eq!(
        say(&error(FileErrorKind::NotFound)),
        "File not found: notes/a.md"
    );
}

#[test]
fn a_directory_where_a_file_was_named_says_so() {
    assert_eq!(
        say(&error(FileErrorKind::IsADirectory)),
        "Path is a directory, not a file: notes/a.md"
    );
}

#[test]
fn a_file_where_a_directory_was_named_says_so() {
    assert_eq!(
        say(&error(FileErrorKind::NotADirectory)),
        "Path is not a directory: notes/a.md"
    );
}

#[test]
fn a_conflict_says_the_path_changed_this_round_and_names_the_tool_to_call_again() {
    let error = error(FileErrorKind::Conflict);
    assert_eq!(
        say(&error),
        "notes/a.md was changed, or is being changed, by another call in this round, so it \
         cannot be read in this round. Call Read again in your next message."
    );
    assert_eq!(
        refusal("Grep", "/src", &error),
        "/src was changed, or is being changed, by another call in this round, so it \
         cannot be read in this round. Call Grep again in your next message."
    );
}

#[test]
fn an_io_failure_names_the_path_as_written_and_then_the_hosts_words() {
    let error = FileError::new(FileErrorKind::Io, "timed out: the disk did not answer");
    assert_eq!(
        say(&error),
        "I/O failure on notes/a.md: timed out: the disk did not answer"
    );
}

#[test]
fn a_denied_read_passes_the_policys_reason_through_unchanged() {
    let reason = "Mode::Plan allows only markdown paths, and /secret.rs is not one";
    assert_eq!(say(&FileError::new(FileErrorKind::Denied, reason)), reason);
    let approval = "Reading /vault needs approval (rule: vault)";
    assert_eq!(
        say(&FileError::new(FileErrorKind::Denied, approval)),
        approval
    );
}

#[test]
fn a_failure_of_no_other_kind_reads_as_the_hosts_words() {
    for text in [
        "path contains a backslash",
        "the run or tool call that owned this access has ended, so /resolved/a.md can no \
         longer be touched through it",
        "the store is offline",
        "/resolved/a.md is a link to /etc/passwd, which is outside the root",
    ] {
        assert_eq!(say(&FileError::new(FileErrorKind::Other, text)), text);
    }
}

#[test]
fn a_relative_path_is_echoed_as_written_and_never_as_the_resolved_one() {
    let error = FileError::new(FileErrorKind::NotFound, "not found: /notes/a.md");
    let text = say(&error);
    assert!(text.ends_with("notes/a.md"), "{text}");
    assert!(!text.contains("/notes/a.md"), "{text}");
}

#[test]
fn no_message_names_a_tool_other_than_the_one_called() {
    let others = ["Write", "StrReplace", "Delete", "Glob", "Grep", "Shell"];
    for kind in [
        FileErrorKind::NotFound,
        FileErrorKind::IsADirectory,
        FileErrorKind::NotADirectory,
        FileErrorKind::Conflict,
        FileErrorKind::Io,
        FileErrorKind::Denied,
        FileErrorKind::Other,
    ] {
        let text = say(&error(kind));
        for other in others {
            assert!(!text.contains(other), "{kind:?} says {text}");
        }
    }
    assert!(!neither_file_nor_directory("p").contains("Shell"));
}

#[test]
fn a_root_that_does_not_exist_says_the_path_does_not_exist() {
    assert_eq!(
        missing_root("/home/dev/gone"),
        "Path does not exist: /home/dev/gone"
    );
    assert_eq!(missing_root("gone"), "Path does not exist: gone");
}

#[test]
fn a_missing_root_is_worded_apart_from_a_missing_file() {
    // The `NotFound` sentence says file, which is wrong for a directory
    // the model named, so the listing and search tools word the root
    // themselves.
    assert_eq!(
        say(&error(FileErrorKind::NotFound)),
        "File not found: notes/a.md"
    );
    assert_eq!(missing_root(WRITTEN), "Path does not exist: notes/a.md");
}

#[test]
fn the_not_a_directory_sentence_is_the_one_the_refusal_gives() {
    assert_eq!(
        say(&error(FileErrorKind::NotADirectory)),
        not_a_directory(WRITTEN)
    );
    assert_eq!(not_a_directory("/a/b"), "Path is not a directory: /a/b");
}

#[test]
fn the_root_sentences_name_no_tool() {
    for text in [missing_root("/p"), not_a_directory("/p")] {
        for tool in [
            "Read",
            "Write",
            "StrReplace",
            "Delete",
            "Glob",
            "Grep",
            "Shell",
        ] {
            assert!(!text.contains(tool), "{text}");
        }
    }
}

#[test]
fn a_special_file_says_it_is_neither_a_file_nor_a_directory() {
    assert_eq!(
        neither_file_nor_directory("/dev/null"),
        "Path is neither a file nor a directory: /dev/null"
    );
}
