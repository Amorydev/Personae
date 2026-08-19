// Default-terminal preference for CLI launches. `cli::launch` (Windows) reads
// `get_default_terminal()` to decide what opens: cmd (today's default),
// PowerShell, or Windows Terminal. Persisted next to `workspaces.json` in the
// same per-OS settings root `cli::config_root()` already exposes.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct TerminalApp {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Default)]
struct TerminalSettings {
    default_terminal: Option<String>,
    /// Path to a user-picked terminal executable, used when
    /// `default_terminal == Some("custom")`.
    custom_path: Option<String>,
}

fn settings_file() -> PathBuf {
    crate::cli::config_root().join("terminal-settings.json")
}

fn load_settings_at(f: &Path) -> TerminalSettings {
    std::fs::read_to_string(f).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings_at(f: &Path, s: &TerminalSettings) -> Result<(), String> {
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let body = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(f, body).map_err(|e| e.to_string())
}

pub fn get_default_terminal() -> Option<String> {
    load_settings_at(&settings_file()).default_terminal
}

/// `None` clears the preference (the "ask me next time" / remove affordance).
pub fn set_default_terminal(id: Option<String>) -> Result<(), String> {
    let f = settings_file();
    let mut s = load_settings_at(&f);
    s.default_terminal = id;
    save_settings_at(&f, &s)
}

pub fn get_custom_terminal_path() -> Option<String> {
    load_settings_at(&settings_file()).custom_path
}

/// Sets the custom terminal path AND selects it as the default in one write,
/// so picking a program from the UI immediately becomes the active choice.
pub fn set_custom_terminal(path: String) -> Result<(), String> {
    let f = settings_file();
    let mut s = load_settings_at(&f);
    s.default_terminal = Some("custom".to_string());
    s.custom_path = Some(path);
    save_settings_at(&f, &s)
}

/// Native "pick a terminal executable" file dialog. Windows only for now — the
/// only OS where this app doesn't already offer a closed, exhaustive terminal
/// list. Mirrors `ide::imp_win::pick_folder`'s WinForms-over-hidden-PowerShell
/// pattern, swapped to an OpenFileDialog filtered to `.exe`.
#[cfg(windows)]
pub fn pick_terminal_exe() -> Result<Option<String>, String> {
    let script = "Add-Type -AssemblyName System.Windows.Forms; \
                  $f=New-Object System.Windows.Forms.OpenFileDialog; \
                  $f.Filter='Programs (*.exe)|*.exe|All files (*.*)|*.*'; \
                  $f.Title='Choose a terminal program'; \
                  if($f.ShowDialog() -eq 'OK'){Write-Output $f.FileName}";
    let (ok, out) = crate::platform::run_powershell_hidden(
        &["-STA", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
    );
    if !ok { return Err(if out.trim().is_empty() { "Program picker failed to run.".into() } else { out.trim().to_string() }); }
    let s = out.trim();
    if s.is_empty() { Ok(None) } else { Ok(Some(s.to_string())) }
}

#[cfg(not(windows))]
pub fn pick_terminal_exe() -> Result<Option<String>, String> {
    Err("Choosing a custom terminal program is Windows-only for now.".into())
}

#[cfg(windows)]
fn windows_terminal_path() -> Option<PathBuf> {
    let (ok, out) = crate::platform::run("where", &["wt"]);
    if ok {
        if let Some(p) = out.lines().map(str::trim).filter(|l| !l.is_empty())
            .map(PathBuf::from).find(|p| p.is_file())
        {
            return Some(p);
        }
    }
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let p = PathBuf::from(local).join("Microsoft").join("WindowsApps").join("wt.exe");
    if p.is_file() { Some(p) } else { None }
}

#[cfg(windows)]
pub fn windows_terminal_available() -> bool { windows_terminal_path().is_some() }

#[cfg(windows)]
pub fn detect_terminals() -> Vec<TerminalApp> {
    let mut out = vec![
        TerminalApp { id: "cmd".into(), name: "Command Prompt".into() },
        TerminalApp { id: "powershell".into(), name: "PowerShell".into() },
    ];
    if windows_terminal_available() {
        out.push(TerminalApp { id: "windows-terminal".into(), name: "Windows Terminal".into() });
    }
    out
}

#[cfg(target_os = "macos")]
pub fn detect_terminals() -> Vec<TerminalApp> {
    let mut out = vec![TerminalApp { id: "terminal".into(), name: "Terminal".into() }];
    for (id, name, app) in [("iterm", "iTerm", "iTerm.app"), ("warp", "Warp", "Warp.app")] {
        if PathBuf::from("/Applications").join(app).exists() {
            out.push(TerminalApp { id: id.into(), name: name.into() });
        }
    }
    out
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn detect_terminals() -> Vec<TerminalApp> { vec![] }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_terminal_roundtrips_and_clears() {
        let f = std::env::temp_dir().join(format!("terminal-settings-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);

        assert_eq!(load_settings_at(&f).default_terminal, None);
        save_settings_at(&f, &TerminalSettings { default_terminal: Some("powershell".into()), custom_path: None }).unwrap();
        assert_eq!(load_settings_at(&f).default_terminal.as_deref(), Some("powershell"));
        save_settings_at(&f, &TerminalSettings { default_terminal: None, custom_path: None }).unwrap();
        assert_eq!(load_settings_at(&f).default_terminal, None);

        std::fs::remove_file(&f).ok();
    }

    #[test]
    fn custom_terminal_sets_default_and_path_together() {
        let f = std::env::temp_dir().join(format!("terminal-settings-custom-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);

        let mut s = load_settings_at(&f);
        s.default_terminal = Some("custom".into());
        s.custom_path = Some(r"C:\tools\wezterm.exe".into());
        save_settings_at(&f, &s).unwrap();

        let loaded = load_settings_at(&f);
        assert_eq!(loaded.default_terminal.as_deref(), Some("custom"));
        assert_eq!(loaded.custom_path.as_deref(), Some(r"C:\tools\wezterm.exe"));

        std::fs::remove_file(&f).ok();
    }

    #[test]
    fn detect_terminals_always_includes_a_baseline() {
        // cmd/PowerShell (Windows) or Terminal (macOS) are unconditional entries;
        // only the extra probed apps (Windows Terminal / iTerm / Warp) vary by host.
        let terminals = detect_terminals();
        #[cfg(any(windows, target_os = "macos"))]
        assert!(!terminals.is_empty());
        #[cfg(not(any(windows, target_os = "macos")))]
        assert!(terminals.is_empty());
    }
}
