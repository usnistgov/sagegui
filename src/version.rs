//! Version information for SageGUI and the embedded Sage engine.
//!
//! This file is the single source of truth for Sage version information.
//! Update these constants when the vendored Sage in `vendor/sage` moves to a
//! new upstream commit (see `vendor/sage/VENDORED.md` and MAINTENANCE.md).

/// The nearest upstream Sage release tag at or before the vendored commit.
/// The README badge reads this value, so it must stay a real tag name.
#[allow(dead_code)]
pub const SAGE_VERSION: &str = "v0.15.0-beta.2";

/// The vendored commit described relative to that tag, in `git describe` form.
/// The vendored engine is 10 upstream commits past `v0.15.0-beta.2`, so this is
/// the honest label to show a user.
pub const SAGE_DESCRIBE: &str = "v0.15.0-beta.2-10-gd74024d";

/// The upstream lazear/sage commit the vendored source is based on
#[allow(dead_code)]
pub const SAGE_COMMIT: &str = "d74024df774054fa411a9d5cca6013ce91d26208";

/// Short commit hash for display
#[allow(dead_code)]
pub const SAGE_COMMIT_SHORT: &str = "d74024df";

/// URL to the Sage release named by `SAGE_VERSION`
#[allow(dead_code)]
pub const SAGE_RELEASE_URL: &str = "https://github.com/lazear/sage/releases/tag/v0.15.0-beta.2";

/// URL to the upstream commit the vendored source is based on
#[allow(dead_code)]
pub const SAGE_COMMIT_URL: &str =
    "https://github.com/lazear/sage/commit/d74024df774054fa411a9d5cca6013ce91d26208";
