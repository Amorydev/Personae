mod platform;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;
mod cli;
mod ide;

use platform::{active, Platform, Profile};

#[tauri::command]
fn claude_found() -> bool { active().claude_found() }

#[tauri::command]
fn list_profiles() -> Vec<Profile> { active().list() }

#[tauri::command]
fn create_profile(name: String, color: Option<String>, isolation: Option<String>) -> Result<(), String> {
    active().create(&name, color, isolation.as_deref().unwrap_or("env"))
}

#[tauri::command]
fn launch_profile(name: String) -> Result<(), String> { active().launch(&name) }

#[tauri::command]
fn quit_profile(name: String) -> Result<(), String> { active().quit(&name) }

#[tauri::command]
fn delete_profile(name: String, purge: bool) -> Result<(), String> { active().delete(&name, purge) }

#[tauri::command]
fn repair_profiles() -> Result<usize, String> { active().repair() }

#[tauri::command]
fn set_profile_color(name: String, color: String) -> Result<(), String> { active().set_color(&name, &color) }

#[tauri::command]
fn reveal_path(path: String) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&path).spawn();
}

#[tauri::command]
fn open_url(url: String) {
    // Opens http(s) / mailto in the user's default handler.
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(windows)]
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", &url]).spawn();
}

#[tauri::command]
fn cli_available() -> bool { cli::available() }

#[tauri::command]
fn list_cli_profiles() -> Vec<cli::CliProfile> { cli::list() }

#[tauri::command]
fn create_cli_profile(name: String) -> Result<(), String> { cli::create(&name) }

#[tauri::command]
fn set_cli_token(name: String, token: String) -> Result<(), String> { cli::set_token(&name, &token) }

#[tauri::command]
fn open_cli_setup_token(name: String) -> Result<(), String> { cli::open_setup_token(&name) }

#[tauri::command]
fn capture_cli_token(name: String) -> Result<Option<String>, String> { cli::capture_token(&name) }

#[tauri::command]
fn launch_cli_profile(name: String) -> Result<(), String> { cli::launch(&name) }

#[tauri::command]
fn delete_cli_profile(name: String, purge: bool) -> Result<(), String> { cli::delete(&name, purge) }

#[tauri::command]
fn list_ides() -> Vec<ide::Ide> { ide::list_ides() }

#[tauri::command]
fn pick_folder() -> Result<Option<String>, String> { ide::pick_folder() }

#[tauri::command]
fn open_in_ide(account: String, ide_id: String, project_path: String, mode: String) -> Result<(), String> {
    ide::open_in_ide(&account, &ide_id, &project_path, &mode)
}

#[tauri::command]
fn list_workspaces() -> Vec<ide::Workspace> { ide::list_workspaces() }

#[tauri::command]
fn save_workspace(account_slug: String, account_name: String, ide_id: String, ide_name: String,
                  project_path: String, mode: String, now: u64) -> Result<(), String> {
    ide::save_workspace(&account_slug, &account_name, &ide_id, &ide_name, &project_path, &mode, now)
}

#[tauri::command]
fn delete_workspace(id: String) -> Result<(), String> { ide::delete_workspace(&id) }

#[tauri::command]
fn open_workspace(id: String, now: u64) -> Result<(), String> { ide::open_workspace(&id, now) }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            claude_found, list_profiles, create_profile,
            launch_profile, quit_profile, delete_profile, repair_profiles,
            set_profile_color, reveal_path, open_url,
            cli_available, list_cli_profiles, create_cli_profile, set_cli_token,
            open_cli_setup_token, capture_cli_token, launch_cli_profile, delete_cli_profile,
            list_ides, pick_folder, open_in_ide,
            list_workspaces, save_workspace, delete_workspace, open_workspace
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
