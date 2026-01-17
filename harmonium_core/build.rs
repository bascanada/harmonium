//! Build script for harmonium_core
//!
//! Captures git version information at compile time to avoid runtime git dependencies.
//! This makes the binary portable and doesn't require git to be installed at runtime.

use std::process::Command;

fn main() {
    // Tell cargo to re-run this if git HEAD changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads/");
    println!("cargo:rerun-if-changed=../.git/refs/tags/");

    // Get git describe (tag + commits since tag)
    let git_tag = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    // Get short SHA
    let git_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Set environment variables for use with env!() macro
    println!("cargo:rustc-env=GIT_VERSION_TAG={}", git_tag);
    println!("cargo:rustc-env=GIT_VERSION_SHA={}", git_sha);
}
