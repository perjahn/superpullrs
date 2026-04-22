#![allow(dead_code, unused_imports)]

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

mod docker_helpers;

#[test]
fn integration_tests_marker() {
    // This is a marker test to indicate integration tests are available
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_ok() {
        println!("Integration tests are enabled");
    } else {
        println!("Set SUPERPULL_INTEGRATION_TESTS=1 to run integration tests");
    }
}
