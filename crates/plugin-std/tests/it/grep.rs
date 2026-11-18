//! Grep's steps, seen from the Host's side of the contract: a search ends
//! its step at each file operation and at each pause, so the Host can
//! abandon it, and an abandoned search leaves the Pack instance usable.

use plugin_std::Std;
use runvil_plugin_sdk::testing::{self, Files};
use serde_json::{Value, json};

use crate::tools::PACK;

/// How many files the step check seeds.
const FILES: usize = 1000;

/// A tree with `FILES` small files, each holding one matching line.
fn seeded() -> Files {
    (0..FILES).fold(Files::new(), |files, index| {
        files.file(&format!("/f{index:04}.txt"), "x\n")
    })
}

/// The search arguments the step check uses: one count row per file.
fn args() -> String {
    let args: Value = json!({ "pattern": "x", "output_mode": "count" });
    args.to_string()
}

#[test]
fn a_search_over_a_thousand_files_ends_a_step_at_each_file_operation_and_each_pause() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(seeded());
    let mut call = pack.start("grep", &args());
    let mut pending = 0;
    let result = loop {
        match call.step().expect("each step follows the contract") {
            Some(result) => break result,
            None => pending += 1,
        }
    };
    assert_eq!(
        pending,
        2 + FILES + FILES / 32,
        "a step ends at the stat of the root, at its listing, at the read of each file, and \
         at the pause after each 32 files"
    );
    let output = result.expect("the search completes with its result");
    assert!(!output.is_trusted());
    let rows: Vec<&str> = output.text().lines().collect();
    assert_eq!(
        rows.len(),
        FILES,
        "every file has its row and there is no notice"
    );
    assert_eq!(rows.first(), Some(&"/f0000.txt:1"));
    assert_eq!(rows.last(), Some(&"/f0999.txt:1"));
}

#[test]
fn a_file_that_grows_past_the_limit_after_its_listing_is_skipped_and_not_searched() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(Files::new().file("/grew.txt", "x\n"));
    let mut call = pack.start("grep", &args());
    // The stat of the root, and then its listing, which says 2 bytes.
    assert_eq!(call.step(), Ok(None));
    assert_eq!(call.step(), Ok(None));
    // Another task makes the file longer than a search takes, before the
    // search asks for its bytes.
    pack.files()
        .write("/grew.txt", vec![b'x'; 16_777_218])
        .expect("the file is written");
    let result = loop {
        if let Some(result) = call.step().expect("each step follows the contract") {
            break result;
        }
    };
    let output = result.expect("the search completes with its result");
    assert_eq!(
        output.text(),
        "No matches found. Searched 0 files.\n\n\
         [Skipped 1 entry: 1 file over 16777216 bytes. First: /grew.txt]\n\
         [No ignore rules ran: .gitignore is not applied, and only entries named .git are \
         skipped.]"
    );
}

/// How many steps a search of the seeded tree takes up to its first
/// pause: the stat of the root, its listing, the reads of 32 files, and
/// the step that ends at the pause.
const FIRST_PAUSE: usize = 2 + 32 + 1;

#[test]
fn abandoning_a_search_at_a_request_or_at_its_first_pause_leaves_the_instance_usable() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(seeded());
    // The search is abandoned at the stat of the root, then at the read
    // of a file with some files searched, and then at its first pause,
    // with 32 files searched and no request with the Host.
    for steps in [1, 10, FIRST_PAUSE] {
        let mut abandoned = pack.start("grep", &args());
        for step in 1..=steps {
            assert_eq!(
                abandoned.step(),
                Ok(None),
                "step {step} of {steps} ends before the search can finish"
            );
        }
        // Dropping the call sends `RV_TOOL_CANCEL`, as a canceled run does.
    }
    let read = pack
        .call("read", r#"{"path":"/f0000.txt"}"#)
        .expect("the call follows the contract")
        .expect("the files still read after the cancel");
    assert_eq!(read.text(), "     1|x\n");
    let again = pack
        .call("grep", &args())
        .expect("the call follows the contract")
        .expect("a fresh search runs to its result");
    assert_eq!(again.text().lines().count(), FILES);
}
