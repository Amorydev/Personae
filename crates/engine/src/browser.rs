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
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct BrowserApp {
    pub id: String,
    pub name: String,
}

/// One Chromium profile inside a browser's user-data dir. `dir` is what goes
/// into `--profile-directory` ("Default", "Profile 1", ...); `name` and
/// `account` are the human labels Chrome shows in its own profile switcher.
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct BrowserProfile {
    pub dir: String,
    pub name: String,
    pub account: Option<String>,
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
    /// CLI account slug -> Chromium profile directory to sign that account in
    /// with. Per-account on purpose: the whole point of Personae is that each
    /// account is a *different* claude.ai login, so they generally live in
    /// different browser profiles and a single global choice cannot serve them.
    /// Absent for accounts never explicitly assigned one.
    #[serde(default)]
    account_profiles: BTreeMap<String, String>,
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
    let existing = load_settings_at(&settings_file());
    save_settings_at(&settings_file(), &BrowserSettings {
        browser_id, custom_path, reuse_profile: Some(reuse_profile),
        // Preserve per-account choices; this call only edits the global prefs.
        account_profiles: existing.account_profiles,
    })
}

/// The browser profile chosen for one CLI account, if any.
pub fn get_account_profile(slug: &str) -> Option<String> {
    load_settings_at(&settings_file()).account_profiles.get(slug).cloned()
}

/// Remember (or, with `None`, forget) which browser profile signs this account
/// in. Everything else in the settings file is left untouched.
pub fn set_account_profile(slug: &str, profile_dir: Option<String>) -> Result<(), String> {
    let mut s = load_settings_at(&settings_file());
    match profile_dir {
        Some(d) => { s.account_profiles.insert(slug.to_string(), d); }
        None => { s.account_profiles.remove(slug); }
    }
    save_settings_at(&settings_file(), &s)
}

/// Parse the `profile.info_cache` map out of a Chromium `Local State` body.
/// Sorted by directory for a stable picker order. Malformed or absent input
/// yields an empty list rather than an error: a browser with no profile
/// metadata is a normal state, not a failure.
fn parse_profiles(local_state: &str) -> Vec<BrowserProfile> {
    let v: serde_json::Value = match serde_json::from_str(local_state) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let cache = match v.pointer("/profile/info_cache").and_then(|c| c.as_object()) {
        Some(c) => c,
        None => return vec![],
    };
    let mut out: Vec<BrowserProfile> = cache.iter().map(|(dir, info)| BrowserProfile {
        dir: dir.clone(),
        name: info.get("name").and_then(|n| n.as_str())
            .filter(|s| !s.is_empty()).unwrap_or(dir).to_string(),
        account: info.get("user_name").and_then(|u| u.as_str())
            .filter(|s| !s.is_empty()).map(String::from),
    }).collect();
    out.sort_by(|a, b| a.dir.cmp(&b.dir));
    out
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

/// Where a Chromium browser keeps `Local State` and its profile directories.
#[cfg(windows)]
fn user_data_root(browser_id: &str) -> Option<PathBuf> {
    let lad = std::env::var("LOCALAPPDATA").map(PathBuf::from).ok()?;
    match browser_id {
        "chrome" => Some(lad.join(r"Google\Chrome\User Data")),
        "edge" => Some(lad.join(r"Microsoft\Edge\User Data")),
        _ => None,
    }
}

/// The system default browser as an id we understand, read from the UserChoice
/// ProgId ("ChromeHTML" / "MSEdgeHTM"). `None` for anything else — Firefox and
/// friends have no `--profile-directory` equivalent, so there is nothing to
/// offer. This matters because leaving the browser preference on "system
/// default" is the common case, and the profile picker has to work there too.
#[cfg(windows)]
fn default_browser_id() -> Option<String> {
    let (ok, out) = crate::platform::run_powershell_hidden(&[
        "-NoProfile", "-Command",
        "(Get-ItemProperty 'HKCU:\\Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice' -Name ProgId -ErrorAction SilentlyContinue).ProgId",
    ]);
    if !ok { return None; }
    let progid = out.trim().to_ascii_lowercase();
    if progid.starts_with("chrome") { Some("chrome".into()) }
    else if progid.starts_with("msedge") { Some("edge".into()) }
    else { None }
}

/// The browser sign-in will actually use: the explicit preference when set,
/// otherwise whatever Windows would have opened anyway.
#[cfg(windows)]
fn effective_browser_id() -> Option<String> {
    match get_prefs().browser_id {
        Some(id) if id != "custom" => Some(id),
        Some(_) => None, // a custom exe: we cannot know its profile layout
        None => default_browser_id(),
    }
}

/// Profiles available in the browser that sign-in will use. Empty when that is
/// a custom exe or a non-Chromium browser, which the caller reads as "nothing
/// to pick" and carries on exactly as before.
#[cfg(windows)]
pub fn list_profiles() -> Vec<BrowserProfile> {
    let root = match effective_browser_id().as_deref().and_then(user_data_root) {
        Some(r) => r,
        None => return vec![],
    };
    std::fs::read_to_string(root.join("Local State"))
        .map(|s| parse_profiles(&s))
        .unwrap_or_default()
}

#[cfg(not(windows))]
pub fn list_profiles() -> Vec<BrowserProfile> { vec![] }

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
fn exe_for_id(id: &str, custom: Option<String>) -> Option<PathBuf> {
    match id {
        "chrome" => chrome_path(),
        "edge" => edge_path(),
        "custom" => custom.map(PathBuf::from),
        _ => None,
    }
}

#[cfg(windows)]
fn resolve_browser() -> Option<(PathBuf, bool)> {
    let prefs = get_prefs();
    let exe = exe_for_id(prefs.browser_id.as_deref()?, prefs.custom_path)?;
    Some((exe, !prefs.reuse_profile))
}

/// Body of the wrapper `.cmd` that `BROWSER` points at. `claude auth login`
/// only ever appends the URL (`<$BROWSER> <url>`), so any flag we need has to
/// be baked in ahead of it. `--guest` wins over a profile when both are asked
/// for: a guest window has no profile by definition.
#[cfg(any(windows, test))]
fn wrapper_body(exe: &str, guest: bool, profile_dir: Option<&str>) -> String {
    let flags = if guest {
        " --guest".to_string()
    } else {
        match profile_dir {
            Some(d) => format!(" --profile-directory=\"{d}\""),
            None => String::new(),
        }
    };
    format!("@echo off\r\nstart \"\" \"{exe}\"{flags} \"%~1\"\r\n")
}

/// Writes a wrapper `.cmd` (if a specific browser + fresh-session flag is
/// needed) and returns the value `BROWSER` should be set to in the login
/// script, or `None` to leave `BROWSER` unset (system default handles it).
/// A plain browser path is enough for "reuse profile" (no flag needed); a
/// wrapper is only needed to inject `--guest` for "new session", since
/// `claude auth login` only ever appends the URL as `<$BROWSER> <url>`.
#[cfg(windows)]
pub fn prepare_login_browser(login_dir: &Path, slug: &str) -> Option<String> {
    let profile_dir = get_account_profile(slug);
    // A chosen profile is reason enough to take over BROWSER even when the
    // preference is still "system default": resolve whatever Windows would
    // have opened so the flag has something to attach to. Without a profile
    // this stays exactly as it was — no preference, no override.
    let (exe, want_guest) = match resolve_browser() {
        Some(r) => r,
        None => match profile_dir.as_deref() {
            Some(_) => (exe_for_id(&default_browser_id()?, None)?, false),
            None => return None,
        },
    };
    if !want_guest && profile_dir.is_none() {
        return Some(exe.display().to_string());
    }
    let wrapper = login_dir.join("browser-launcher.cmd");
    let body = wrapper_body(&exe.display().to_string(), want_guest, profile_dir.as_deref());
    std::fs::create_dir_all(login_dir).ok()?;
    std::fs::write(&wrapper, body).ok()?;
    Some(wrapper.display().to_string())
}

#[cfg(not(windows))]
pub fn prepare_login_browser(_login_dir: &Path, _slug: &str) -> Option<String> { None }

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
            account_profiles: BTreeMap::new(),
        }).unwrap();
        let loaded = load_settings_at(&f);
        assert_eq!(loaded.browser_id.as_deref(), Some("chrome"));
        assert_eq!(loaded.reuse_profile, Some(false));

        std::fs::remove_file(&f).ok();
    }

    /// A settings file written before per-account profiles existed must still
    /// load — otherwise everyone's browser preference silently resets.
    #[test]
    fn settings_without_account_profiles_still_load() {
        let f = std::env::temp_dir().join(format!("browser-legacy-{}.json", std::process::id()));
        std::fs::write(&f, r#"{"browser_id":"edge","custom_path":null,"reuse_profile":true}"#).unwrap();
        let loaded = load_settings_at(&f);
        assert_eq!(loaded.browser_id.as_deref(), Some("edge"));
        assert!(loaded.account_profiles.is_empty());
        std::fs::remove_file(&f).ok();
    }

    #[test]
    fn account_profiles_roundtrip() {
        let f = std::env::temp_dir().join(format!("browser-accts-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);
        let mut m = BTreeMap::new();
        m.insert("justin".to_string(), "Profile 1".to_string());
        save_settings_at(&f, &BrowserSettings {
            browser_id: None, custom_path: None, reuse_profile: None, account_profiles: m,
        }).unwrap();
        let loaded = load_settings_at(&f);
        assert_eq!(loaded.account_profiles.get("justin").map(String::as_str), Some("Profile 1"));
        assert_eq!(loaded.account_profiles.get("nobody"), None);
        std::fs::remove_file(&f).ok();
    }

    #[test]
    fn wrapper_quotes_the_url_arg() {
        let body = wrapper_body(r"C:\Program Files\Google\Chrome\Application\chrome.exe", true, None);
        assert!(body.contains("--guest \"%~1\""), "URL arg must be quoted: {body}");
        assert!(!body.contains("--guest %1"), "must not use a bare %1: {body}");
    }

    #[test]
    fn wrapper_passes_profile_directory_quoted() {
        let body = wrapper_body(r"C:\chrome.exe", false, Some("Profile 1"));
        // The space in "Profile 1" is exactly why this has to be quoted.
        assert!(body.contains("--profile-directory=\"Profile 1\""), "{body}");
        assert!(body.contains("\"%~1\""), "URL arg must still be quoted: {body}");
        assert!(!body.contains("--guest"), "profile mode must not go guest: {body}");
    }

    #[test]
    fn wrapper_without_flags_is_a_plain_launch() {
        let body = wrapper_body(r"C:\chrome.exe", false, None);
        assert!(!body.contains("--guest"), "{body}");
        assert!(!body.contains("--profile-directory"), "{body}");
        assert!(body.contains("\"%~1\""), "{body}");
    }

    /// Guest wins: a guest window has no profile, so honouring both would
    /// silently produce a window that is neither.
    #[test]
    fn guest_beats_profile_when_both_are_set() {
        let body = wrapper_body(r"C:\chrome.exe", true, Some("Profile 1"));
        assert!(body.contains("--guest"), "{body}");
        assert!(!body.contains("--profile-directory"), "{body}");
    }

    #[test]
    fn parses_chromium_local_state_profiles() {
        let body = r#"{"profile":{"last_used":"Default","info_cache":{
            "Profile 1":{"name":"Syd","user_name":"syd@example.com"},
            "Default":{"name":"First user","user_name":"first@example.com"},
            "Profile 2":{"name":"","user_name":""}
        }}}"#;
        let got = parse_profiles(body);
        // Sorted by directory, so the picker order does not jump around.
        assert_eq!(got.iter().map(|p| p.dir.as_str()).collect::<Vec<_>>(),
                   vec!["Default", "Profile 1", "Profile 2"]);
        assert_eq!(got[1].name, "Syd");
        assert_eq!(got[1].account.as_deref(), Some("syd@example.com"));
        // Blank name falls back to the directory; blank account becomes None.
        assert_eq!(got[2].name, "Profile 2");
        assert_eq!(got[2].account, None);
    }

    #[test]
    fn malformed_local_state_yields_no_profiles() {
        assert!(parse_profiles("not json").is_empty());
        assert!(parse_profiles("{}").is_empty());
        assert!(parse_profiles(r#"{"profile":{"info_cache":[]}}"#).is_empty());
    }
}
