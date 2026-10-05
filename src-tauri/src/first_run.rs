//! The first-run warning every ansiapps app shows (Diskette's
//! `src-tauri/src/first_run.rs` is the reference): before anything else
//! happens (no window, no launch screen), a native
//! dialog says the app is still in development and used at your own
//! risk. "OK" starts the app and is remembered; "I'll Be Back." quits
//! without creating anything, so the warning shows again next launch.

use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// Written to the app-data dir once the user presses OK.
const ACCEPTED_MARKER: &str = "in-development-accepted";

pub fn accepted(app_data_dir: &Path) -> bool {
    app_data_dir.join(ACCEPTED_MARKER).exists()
}

fn remember(app_data_dir: &Path) {
    if let Err(e) = std::fs::create_dir_all(app_data_dir)
        .and_then(|_| std::fs::write(app_data_dir.join(ACCEPTED_MARKER), b""))
    {
        // Not fatal: the warning just shows again next launch.
        log::warn!("couldn't remember the first-run warning was accepted: {e}");
    }
}

/// Shows the warning, then calls `start` on OK or quits on "I'll Be Back.".
/// Non-blocking: the dialog appears once the event loop runs.
pub fn ask(app: &AppHandle, app_data_dir: &Path, start: impl FnOnce(&AppHandle) + Send + 'static) {
    let handle = app.clone();
    let dir = app_data_dir.to_path_buf();
    app.dialog()
        .message(
            "Coupler is still in development. Some features may be missing or \
             not work as expected. Use it at your own risk.",
        )
        .title("Welcome to Coupler")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("OK".into(), "I'll Be Back.".into()))
        .show(move |ok| {
            if ok {
                remember(&dir);
                start(&handle);
            } else {
                handle.exit(0);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_only_after_remember() {
        let dir = std::env::temp_dir().join(format!("coupler-first-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!accepted(&dir));
        // A missing app-data dir (a true first run) is created.
        remember(&dir);
        assert!(accepted(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
