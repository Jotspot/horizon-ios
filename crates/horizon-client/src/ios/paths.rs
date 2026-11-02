//! Where the iOS app keeps its files (docs/ios.md). Everything is under the app's `Documents/`,
//! which the Files app shows ("On My iPhone → Horizon") because of `UIFileSharingEnabled`.
//!
//! ```text
//! Documents/
//!   horizon-client/26.3/   the prepared resources the player copies in
//!   horizon/               the game folder and home: saves, options.txt, logs, crash reports
//! ```
use std::path::{Path, PathBuf};

/// The app's `Documents/`. iOS sets `HOME` to the app's sandbox container.
pub fn documents() -> PathBuf {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("Documents")
}

/// The game folder (the working directory) and its home: logs and crash reports.
pub fn game_dir(documents: &Path) -> PathBuf {
    documents.join("horizon")
}

/// Where the player puts their prepared `.local/horizon-client/26.3`.
pub fn resources(documents: &Path) -> PathBuf {
    documents
        .join("horizon-client")
        .join(horizon_core::VANILLA_VERSION)
}

/// What the "missing resources" screen and the log say on iOS: where to copy the folder, since
/// nothing can be prepared on the phone.
pub fn missing_resources_hint(looked_in: &Path) -> String {
    format!(
        "Copy the prepared folder .local/horizon-client/{v} from a computer (the desktop Horizon \
         launcher makes it) into Files → On My iPhone → Horizon → horizon-client/{v}. Looked in {}",
        looked_in.display(),
        v = horizon_core::VANILLA_VERSION,
    )
}
