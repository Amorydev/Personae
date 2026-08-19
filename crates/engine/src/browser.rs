// Which browser `claude auth login` opens for CLI sign-in, and whether it
// reuses the browser's existing signed-in profile or starts a fresh session.
// `claude auth login` opens its OAuth URL via `<$BROWSER> <url>` (confirmed
// live: setting BROWSER=notepad.exe made it invoke `notepad.exe <oauth-url>`
// verbatim) — the same convention the `open` npm package uses. Chrome/Edge
// already reuse the default profile's existing login when invoked with just a
// URL, so "reuse" needs no BROWSER override at all; "new session" needs
// `--guest`, which isn't expressible through BROWSER alone, so we point
// BROWSER at a tiny generated wrapper .cmd that adds the flag before the URL.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct BrowserApp {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Default)]
struct BrowserSettings {
    /// `None` = system default (today's behavior, no BROWSER override).
    /// `Some("chrome" | "edge")` = a detected browser. `Some("custom")` = `custom_path`.
    browser_id: Option<String>,
    custom_path: Option<String>,
    /// `true` (default when unset) = reuse the browser's existing signed-in
    /// profile; `false` = force a fresh `--guest` session.
    reuse_profile: Option<bool>,
}

fn settings_file() -> PathBuf {
    crate::cli::config_root().join("browser-settings.json")
}

fn load_settings_at(f: &Path) -> BrowserSettings {
    std::fs::read_to_string(f).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings_at(f: &Path, s: &BrowserSettings) -> Result<(), String> {
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let body = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(f, body).map_err(|e| e.to_string())
}

#[derive(Serialize, Clone)]
pub struct BrowserPrefs {
    pub browser_id: Option<String>,
    pub custom_path: Option<String>,
    pub reuse_profile: bool,
}

pub fn get_prefs() -> BrowserPrefs {
    let s = load_settings_at(&settings_file());
    BrowserPrefs { browser_id: s.browser_id, custom_path: s.custom_path, reuse_profile: s.reuse_profile.unwrap_or(true) }
}

pub fn set_prefs(browser_id: Option<String>, custom_path: Option<String>, reuse_profile: bool) -> Result<(), String> {
    save_settings_at(&settings_file(), &BrowserSettings {
        browser_id, custom_path, reuse_profile: Some(reuse_profile),
    })
}

#[cfg(windows)]
fn candidate(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.is_file()).cloned()
}

#[cfg(windows)]
fn chrome_path() -> Option<PathBuf> {
    let pf = std::env::var("ProgramFiles").map(PathBuf::from).ok();
    let pf86 = std::env::var("ProgramFiles(x86)").map(PathBuf::from).ok();
    let lad = std::env::var("LOCALAPPDATA").map(PathBuf::from).ok();
    let mut paths = vec![];
    for base in [pf, pf86, lad].into_iter().flatten() {
        paths.push(base.join(r"Google\Chrome\Application\chrome.exe"));
    }
    candidate(&paths)
}

#[cfg(windows)]
fn edge_path() -> Option<PathBuf> {
    let pf = std::env::var("ProgramFiles").map(PathBuf::from).ok();
    let pf86 = std::env::var("ProgramFiles(x86)").map(PathBuf::from).ok();
    let mut paths = vec![];
    for base in [pf, pf86].into_iter().flatten() {
        paths.push(base.join(r"Microsoft\Edge\Application\msedge.exe"));
    }
    candidate(&paths)
}

#[cfg(windows)]
pub fn detect_browsers() -> Vec<BrowserApp> {
    let mut out = vec![];
    if chrome_path().is_some() { out.push(BrowserApp { id: "chrome".into(), name: "Google Chrome".into() }); }
    if edge_path().is_some() { out.push(BrowserApp { id: "edge".into(), name: "Microsoft Edge".into() }); }
    out
}

#[cfg(not(windows))]
pub fn detect_browsers() -> Vec<BrowserApp> { vec![] }

#[cfg(windows)]
pub fn pick_browser_exe() -> Result<Option<String>, String> {
    let script = "Add-Type -AssemblyName System.Windows.Forms; \
                  $f=New-Object System.Windows.Forms.OpenFileDialog; \
                  $f.Filter='Programs (*.exe)|*.exe|All files (*.*)|*.*'; \
                  $f.Title='Choose a browser'; \
                  if($f.ShowDialog() -eq 'OK'){Write-Output $f.FileName}";
    let (ok, out) = crate::platform::run_powershell_hidden(
        &["-STA", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
    );
    if !ok { return Err(if out.trim().is_empty() { "Program picker failed to run.".into() } else { out.trim().to_string() }); }
    let s = out.trim();
    if s.is_empty() { Ok(None) } else { Ok(Some(s.to_string())) }
}

#[cfg(not(windows))]
pub fn pick_browser_exe() -> Result<Option<String>, String> {
    Err("Choosing a custom browser is Windows-only for now.".into())
}

/// Resolve the current preference to (browser_exe, extra_flag_for_new_session).
/// Returns `None` for "system default" (don't touch BROWSER at all).
#[cfg(windows)]
fn resolve_browser() -> Option<(PathBuf, bool)> {
    let prefs = get_prefs();
    let exe = match prefs.browser_id.as_deref() {
        Some("chrome") => chrome_path()?,
        Some("edge") => edge_path()?,
        Some("custom") => PathBuf::from(prefs.custom_path?),
        _ => return None,
    };
    Some((exe, !prefs.reuse_profile))
}

#[cfg(any(windows, test))]
fn guest_wrapper_body(exe: &str) -> String {
    format!("@echo off\r\nstart \"\" \"{exe}\" --guest \"%~1\"\r\n")
}

/// Writes a wrapper `.cmd` (if a specific browser + fresh-session flag is
/// needed) and returns the value `BROWSER` should be set to in the login
/// script, or `None` to leave `BROWSER` unset (system default handles it).
/// A plain browser path is enough for "reuse profile" (no flag needed); a
/// wrapper is only needed to inject `--guest` for "new session", since
/// `claude auth login` only ever appends the URL as `<$BROWSER> <url>`.
#[cfg(windows)]
pub fn prepare_login_browser(login_dir: &Path) -> Option<String> {
    let (exe, want_guest) = resolve_browser()?;
    if !want_guest {
        return Some(exe.display().to_string());
    }
    let wrapper = login_dir.join("browser-launcher.cmd");
    let body = guest_wrapper_body(&exe.display().to_string());
    std::fs::create_dir_all(login_dir).ok()?;
    std::fs::write(&wrapper, body).ok()?;
    Some(wrapper.display().to_string())
}

#[cfg(not(windows))]
pub fn prepare_login_browser(_login_dir: &Path) -> Option<String> { None }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_roundtrip_and_default_to_reuse() {
        let f = std::env::temp_dir().join(format!("browser-settings-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);

        assert_eq!(load_settings_at(&f).reuse_profile, None); // unset means "reuse" at the call site
        save_settings_at(&f, &BrowserSettings {
            browser_id: Some("chrome".into()), custom_path: None, reuse_profile: Some(false),
        }).unwrap();
        let loaded = load_settings_at(&f);
        assert_eq!(loaded.browser_id.as_deref(), Some("chrome"));
        assert_eq!(loaded.reuse_profile, Some(false));

        std::fs::remove_file(&f).ok();
    }

    #[test]
    fn guest_wrapper_quotes_the_url_arg() {
        let body = guest_wrapper_body(r"C:\Program Files\Google\Chrome\Application\chrome.exe");
        assert!(body.contains("--guest \"%~1\""), "URL arg must be quoted: {body}");
        assert!(!body.contains("--guest %1"), "must not use a bare %1: {body}");
    }
}
