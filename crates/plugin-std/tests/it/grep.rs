//! Grep's steps, seen from the Host's side of the contract: a search over
//! many files ends its step before it completes, so the Host can abandon
//! it, and an abandoned search leaves the Pack instance usable.

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
fn a_search_over_a_thousand_files_takes_many_steps_before_it_completes() {
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
    assert!(
        pending >= 31,
        "a step ends after each 32 of {FILES} files, and the search took {pending} extra steps"
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
fn abandoning_a_search_after_its_first_step_leaves_the_instance_usable() {
    let loaded = testing::load::<Std>().expect("the Plugin loads");
    let pack = loaded
        .new_pack(PACK)
        .expect("the Pack instance is made")
        .with_files(seeded());
    {
        let mut abandoned = pack.start("grep", &args());
        assert_eq!(
            abandoned.step(),
            Ok(None),
            "the search ends its first step before it can finish"
        );
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
