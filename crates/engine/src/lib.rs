// Personae's platform-agnostic business logic. Deliberately has no dependency
// on tauri/wry — `src-tauri` is a thin wrapper crate that only translates
// these functions into `#[tauri::command]`s. Splitting it out this way means
// `cargo check`/`cargo test` against this crate alone (or the whole workspace
// in parallel) never has to touch the tauri/wry dependency tree, which is
// most of a from-scratch build's compile time.
pub mod platform;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;
pub mod cli;
pub mod ide;
pub mod terminal;
pub mod browser;
pub mod desktop_prefs;
