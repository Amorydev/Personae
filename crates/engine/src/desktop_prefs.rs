// User-overridable Claude Desktop executable path. Field report: the
// auto-detector's AppX/MSIX InstallLocation probe can resolve to a path under
// `C:\Program Files\WindowsApps\...` that looks stat-able but isn't reliably
// launchable outside the package's own activation context — MSIX packages are
// generally meant to be started via their execution alias, not a raw
// `CreateProcess` on the InstallLocation exe. When auto-detection picks a
// stale/wrong copy, this override lets the user point Personae at the working
// claude.exe directly; it takes priority over every auto-detect signal.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default)]
struct DesktopSettings {
    custom_exe: Option<String>,
}

fn settings_file() -> PathBuf {
    crate::cli::config_root().join("desktop-settings.json")
}

fn load_settings_at(f: &Path) -> DesktopSettings {
    std::fs::read_to_string(f).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings_at(f: &Path, s: &DesktopSettings) -> Result<(), String> {
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let body = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(f, body).map_err(|e| e.to_string())
}

pub fn get_custom_exe() -> Option<String> {
    load_settings_at(&settings_file()).custom_exe
}

/// `None` clears the override, returning to auto-detection.
pub fn set_custom_exe(path: Option<String>) -> Result<(), String> {
    save_settings_at(&settings_file(), &DesktopSettings { custom_exe: path })
}

/// Native "pick the Claude Desktop executable" file dialog. Windows only —
/// this override exists specifically for the Windows MSIX-resolution bug.
#[cfg(windows)]
pub fn pick_desktop_exe() -> Result<Option<String>, String> {
    let script = "Add-Type -AssemblyName System.Windows.Forms; \
                  $f=New-Object System.Windows.Forms.OpenFileDialog; \
                  $f.Filter='Programs (*.exe)|*.exe|All files (*.*)|*.*'; \
                  $f.Title='Choose the Claude Desktop executable'; \
                  if($f.ShowDialog() -eq 'OK'){Write-Output $f.FileName}";
    let (ok, out) = crate::platform::run_powershell_hidden(
        &["-STA", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
    );
    if !ok { return Err(if out.trim().is_empty() { "Program picker failed to run.".into() } else { out.trim().to_string() }); }
    let s = out.trim();
    if s.is_empty() { Ok(None) } else { Ok(Some(s.to_string())) }
}

#[cfg(not(windows))]
pub fn pick_desktop_exe() -> Result<Option<String>, String> {
    Err("Choosing a custom Claude Desktop executable is Windows-only for now.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_roundtrip_and_default_none() {
        let f = std::env::temp_dir().join(format!("desktop-settings-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);

        assert_eq!(load_settings_at(&f).custom_exe, None);
        save_settings_at(&f, &DesktopSettings { custom_exe: Some(r"C:\Claude\Claude.exe".into()) }).unwrap();
        assert_eq!(load_settings_at(&f).custom_exe.as_deref(), Some(r"C:\Claude\Claude.exe"));

        std::fs::remove_file(&f).ok();
    }
}
