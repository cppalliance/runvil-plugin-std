//! Integration tests for `plugin-std`, through its entry point and the
//! SDK's fake Host: the Pack the Plugin inserts, the tools one run's
//! instance adds, what a call answers with arguments that match or miss
//! the schema, and what a call that changes files leaves in its run.
#![expect(
    clippy::expect_used,
    reason = "test helpers panic on setup failure, which is the desired behavior"
)]

mod changes;
mod grep;
mod tools;
