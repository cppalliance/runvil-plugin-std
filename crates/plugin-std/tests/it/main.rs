//! Integration tests for `plugin-std`, through its entry point and the
//! SDK's fake Host: the Pack the Plugin inserts, the tools one run's
//! instance adds, and what a call answers with arguments that match or
//! miss the schema.
#![expect(
    clippy::expect_used,
    reason = "test helpers panic on setup failure, which is the desired behavior"
)]

mod grep;
mod tools;
