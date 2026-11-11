//! Unit tests for the shared walk order: the one sequence ListDir and
//! Grep both visit entries in.

use runvil_plugin_sdk::{DirEntry, FileType, Stat};

use super::{GIT, has_control, sorted};

/// An entry called `name` of the given kind.
fn entry(name: &str, file_type: FileType) -> DirEntry {
    DirEntry::new(name, Stat::new(file_type, 0))
}

/// The names of `entries`, in order.
fn names(entries: &[DirEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.name.as_str()).collect()
}

#[test]
fn a_directory_sorts_as_its_name_with_a_slash_added() {
    let entries = vec![
        entry("b", FileType::File),
        entry("a.txt", FileType::File),
        entry("a", FileType::Directory),
        entry("a-b", FileType::File),
        entry("B", FileType::File),
    ];
    // Full paths `/B`, `/a-b`, `/a.txt`, `/a/x`, `/b` sort bytewise in
    // this order: `-` is below `.`, which is below `/`.
    assert_eq!(names(&sorted(entries)), ["B", "a-b", "a.txt", "a", "b"]);
}

#[test]
fn a_directory_and_a_file_that_share_a_prefix_follow_the_path_order() {
    let entries = vec![
        entry("x.rs", FileType::File),
        entry("x", FileType::Directory),
        entry("x-y", FileType::File),
    ];
    assert_eq!(names(&sorted(entries)), ["x-y", "x.rs", "x"]);
}

#[test]
fn the_order_does_not_depend_on_the_order_listed() {
    let forward = vec![
        entry("a", FileType::Directory),
        entry("a.txt", FileType::File),
        entry("b", FileType::File),
    ];
    let backward: Vec<DirEntry> = forward.iter().rev().cloned().collect();
    assert_eq!(names(&sorted(forward)), names(&sorted(backward)));
}

#[test]
fn the_skipped_name_is_dot_git() {
    assert_eq!(GIT, ".git");
}

#[test]
fn a_name_holds_a_control_character_below_a_space_or_at_delete() {
    for name in ["a\nb", "\tx", "a\u{1}", "a\u{1f}b", "a\u{7f}", "\0"] {
        assert!(has_control(name), "{name:?}");
    }
    for name in ["", "a b", "a.txt", "caf\u{e9}", "a\u{80}", "~"] {
        assert!(!has_control(name), "{name:?}");
    }
}
