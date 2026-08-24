//! Version information for SageGUI and the embedded Sage engine.
//!
//! This file is the single source of truth for Sage version information.
//! Update these constants when upgrading to a new Sage version.

/// The Sage engine version tag (e.g., "v0.15.0-beta.2")
pub const SAGE_VERSION: &str = "v0.15.0-beta.2";

/// The Sage commit hash this build is pinned to
#[allow(dead_code)]
pub const SAGE_COMMIT: &str = "ed5f06caae7305d1599bd877758759112e772d20";

/// Short commit hash for display
#[allow(dead_code)]
pub const SAGE_COMMIT_SHORT: &str = "ed5f06ca";

/// URL to the Sage release
#[allow(dead_code)]
pub const SAGE_RELEASE_URL: &str = "https://github.com/lazear/sage/releases/tag/v0.15.0-beta.2";

/// URL to the specific commit in our fork
#[allow(dead_code)]
pub const SAGE_COMMIT_URL: &str =
    "https://github.com/neely/sage/commit/ed5f06caae7305d1599bd877758759112e772d20";
