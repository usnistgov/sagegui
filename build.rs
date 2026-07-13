//! Build script for SageGUI
//!
//! This script extracts the Sage version from Cargo.toml at compile time
//! and makes it available as a compile-time constant.

use std::fs;

fn main() {
    // Tell Cargo to rerun this script if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Extract Sage version from Cargo.toml
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");

    // Look for the sage-core git dependency and extract the rev
    let sage_version = extract_sage_version(&cargo_toml);

    // Set environment variable for use in code
    println!("cargo:rustc-env=SAGE_COMMIT={}", sage_version);

    // Also try to get the Sage tag/version from the comment in Cargo.toml
    let sage_tag = extract_sage_tag(&cargo_toml);
    println!("cargo:rustc-env=SAGE_VERSION={}", sage_tag);
}

fn extract_sage_version(cargo_toml: &str) -> String {
    // Look for: rev = "d74024df..."
    for line in cargo_toml.lines() {
        if line.contains("sage-core") && line.contains("rev =") {
            // Extract the commit hash
            if let Some(start) = line.find("rev = \"") {
                let rest = &line[start + 7..];
                if let Some(end) = rest.find('"') {
                    return rest[..end].to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn extract_sage_tag(cargo_toml: &str) -> String {
    // Look for comment like: # Pinned to v0.15.0-beta.2
    for line in cargo_toml.lines() {
        if line.contains("Pinned to v") || line.contains("# v") {
            // Extract version pattern like v0.15.0-beta.2
            let words: Vec<&str> = line.split_whitespace().collect();
            for word in words {
                if word.starts_with("v") && word.contains('.') {
                    // Clean up any trailing punctuation
                    return word
                        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '-')
                        .to_string();
                }
            }
        }
    }
    "v0.15.0-beta.2".to_string() // Default fallback
}
