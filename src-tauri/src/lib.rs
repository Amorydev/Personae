// All business logic lives in the personae-engine crate (see crates/engine —
// deliberately has no tauri dependency, so it builds/tests independently of
// the GUI stack). This crate is just the tauri-command translation layer.
use personae_engine::{browser, cli, desktop_prefs, ide, platform, terminal};
use platform::{active, Platform, Profile};

/// Tauri v2 runs non-async `#[tauri::command]` handlers on the main UI thread. Every command
/// below delegates its (synchronous, often process-spawning) body to a blocking-pool thread via
/// this helper so a slow spawn (console creation, PowerShell startup, AV scanning, folder
/// picker dialogs) can never freeze the window.
async fn blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .expect("background task panicked")
}

#[tauri::command]
async fn claude_found() -> bool { blocking(|| active().claude_found()).await }

#[tauri::command]
async fn list_profiles() -> Vec<Profile> { blocking(|| active().list()).await }

#[tauri::command]
async fn create_profile(name: String, color: Option<String>, isolation: Option<String>) -> Result<(), String> {
    blocking(move || active().create(&name, color, isolation.as_deref().unwrap_or("env"))).await
}

#[tauri::command]
async fn launch_profile(name: String) -> Result<(), String> { blocking(move || active().launch(&name)).await }

#[tauri::command]
async fn quit_profile(name: String) -> Result<(), String> { blocking(move || active().quit(&name)).await }

#[tauri::command]
async fn delete_profile(name: String, purge: bool) -> Result<(), String> { blocking(move || active().delete(&name, purge)).await }

#[tauri::command]
async fn repair_profiles() -> Result<usize, String> { blocking(|| active().repair()).await }

#[tauri::command]
async fn set_profile_color(name: String, color: String) -> Result<(), String> { blocking(move || active().set_color(&name, &color)).await }

#[tauri::command]
async fn fetch_desktop_usage(name: String) -> (Option<u32>, Option<u32>) { blocking(move || active().fetch_usage(&name)).await }

#[tauri::command]
async fn get_desktop_exe_override() -> Option<String> { blocking(desktop_prefs::get_custom_exe).await }

#[tauri::command]
async fn set_desktop_exe_override(path: Option<String>) -> Result<(), String> {
    blocking(move || desktop_prefs::set_custom_exe(path)).await
}

#[tauri::command]
async fn pick_desktop_exe() -> Result<Option<String>, String> { blocking(desktop_prefs::pick_desktop_exe).await }

#[tauri::command]
async fn reveal_path(path: String) {
    blocking(move || {
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&path).spawn();
        #[cfg(windows)]
        let _ = std::process::Command::new("explorer").arg(&path).spawn();
    })
    .await
}

#[tauri::command]
async fn open_url(url: String) {
    // Opens http(s) / mailto in the user's default handler.
    blocking(move || {
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&url).spawn();
        #[cfg(windows)]
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", &url]).spawn();
    })
    .await
}

#[tauri::command]
async fn cli_available() -> bool { blocking(cli::available).await }

#[tauri::command]
async fn list_cli_profiles() -> Vec<cli::CliProfile> { blocking(cli::list).await }

#[tauri::command]
async fn create_cli_profile(name: String) -> Result<(), String> { blocking(move || cli::create(&name)).await }

#[tauri::command]
async fn rename_cli_profile(old_name: String, new_name: String) -> Result<(), String> {
    blocking(move || cli::rename(&old_name, &new_name)).await
}

#[tauri::command]
async fn login_cli_profile(name: String) -> Result<(), String> { blocking(move || cli::login(&name)).await }

#[tauri::command]
async fn launch_cli_profile(name: String, project_path: Option<String>) -> Result<(), String> {
    blocking(move || cli::launch(&name, project_path.as_deref())).await
}

#[tauri::command]
async fn get_launch_history(name: String) -> Vec<String> { blocking(move || cli::get_launch_history(&name)).await }

#[tauri::command]
async fn fetch_live_cli_usage(name: String) -> Result<(Option<u32>, Option<u32>), String> {
    blocking(move || cli::fetch_live_usage(&name)).await
}

#[tauri::command]
async fn delete_cli_profile(name: String, purge: bool) -> Result<(), String> { blocking(move || cli::delete(&name, purge)).await }

#[tauri::command]
async fn get_cli_provider_config(name: String) -> Result<cli::ProviderConfig, String> {
    blocking(move || cli::get_provider_config(&name)).await
}

#[tauri::command]
async fn set_cli_provider_config(name: String, config: cli::ProviderConfig) -> Result<(), String> {
    blocking(move || cli::set_provider_config(&name, config)).await
}

#[tauri::command]
async fn list_terminals() -> Vec<terminal::TerminalApp> { blocking(terminal::detect_terminals).await }

#[tauri::command]
async fn get_default_terminal() -> Option<String> { blocking(terminal::get_default_terminal).await }

#[tauri::command]
async fn set_default_terminal(id: Option<String>) -> Result<(), String> { blocking(move || terminal::set_default_terminal(id)).await }

#[tauri::command]
async fn get_custom_terminal_path() -> Option<String> { blocking(terminal::get_custom_terminal_path).await }

#[tauri::command]
async fn set_custom_terminal(path: String) -> Result<(), String> { blocking(move || terminal::set_custom_terminal(path)).await }

#[tauri::command]
async fn pick_terminal_exe() -> Result<Option<String>, String> { blocking(terminal::pick_terminal_exe).await }

#[tauri::command]
async fn list_browsers() -> Vec<browser::BrowserApp> { blocking(browser::detect_browsers).await }

#[tauri::command]
async fn get_browser_prefs() -> browser::BrowserPrefs { blocking(browser::get_prefs).await }

#[tauri::command]
async fn set_browser_prefs(browser_id: Option<String>, custom_path: Option<String>, reuse_profile: bool) -> Result<(), String> {
    blocking(move || browser::set_prefs(browser_id, custom_path, reuse_profile)).await
}

#[tauri::command]
async fn pick_browser_exe() -> Result<Option<String>, String> { blocking(browser::pick_browser_exe).await }

#[tauri::command]
async fn list_browser_profiles() -> Vec<browser::BrowserProfile> { blocking(browser::list_profiles).await }

#[tauri::command]
async fn get_account_browser_profile(slug: String) -> Option<String> {
    blocking(move || browser::get_account_profile(&slug)).await
}

#[tauri::command]
async fn set_account_browser_profile(slug: String, profile_dir: Option<String>) -> Result<(), String> {
    blocking(move || browser::set_account_profile(&slug, profile_dir)).await
}

#[tauri::command]
async fn list_ides() -> Vec<ide::Ide> { blocking(ide::list_ides).await }

#[tauri::command]
async fn pick_folder() -> Result<Option<String>, String> { blocking(ide::pick_folder).await }

#[tauri::command]
async fn open_in_ide(account: String, ide_id: String, project_path: String) -> Result<(), String> {
    blocking(move || ide::open_in_ide(&account, &ide_id, &project_path)).await
}

#[tauri::command]
async fn list_workspaces() -> Vec<ide::Workspace> { blocking(ide::list_workspaces).await }

#[tauri::command]
async fn save_workspace(account_slug: String, account_name: String, ide_id: String, ide_name: String,
                  project_path: String, now: u64) -> Result<(), String> {
    blocking(move || ide::save_workspace(&account_slug, &account_name, &ide_id, &ide_name, &project_path, now)).await
}

#[tauri::command]
async fn delete_workspace(id: String) -> Result<(), String> { blocking(move || ide::delete_workspace(&id)).await }

#[tauri::command]
async fn open_workspace(id: String, now: u64) -> Result<(), String> { blocking(move || ide::open_workspace(&id, now)).await }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            claude_found, list_profiles, create_profile,
            launch_profile, quit_profile, delete_profile, repair_profiles,
            set_profile_color, fetch_desktop_usage, get_desktop_exe_override, set_desktop_exe_override, pick_desktop_exe,
            reveal_path, open_url,
            cli_available, list_cli_profiles, create_cli_profile, rename_cli_profile, login_cli_profile,
            launch_cli_profile, get_launch_history, fetch_live_cli_usage, delete_cli_profile,
            get_cli_provider_config, set_cli_provider_config,
            list_terminals, get_default_terminal, set_default_terminal,
            get_custom_terminal_path, set_custom_terminal, pick_terminal_exe,
            list_browsers, get_browser_prefs, set_browser_prefs, pick_browser_exe,
            list_browser_profiles, get_account_browser_profile, set_account_browser_profile,
            list_ides, pick_folder, open_in_ide,
            list_workspaces, save_workspace, delete_workspace, open_workspace
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
