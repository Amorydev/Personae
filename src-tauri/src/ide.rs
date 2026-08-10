// IDE integration: launch a CLI account inside a VS Code-family IDE at a project.
// A Claude Code account is chosen entirely by CLAUDE_CONFIG_DIR (claude itself
// resolves the per-profile login from its own Keychain slot, namespaced by a
// hash of that env var). We just set that one env var in the workspace's
// integrated terminal and auto-open a Claude terminal on folder-open — no
// shims, no PATH rewriting, no app-managed secrets.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct Ide {
    pub id: String,       // "vscode" | "cursor" | "windsurf" | "antigravity"
    pub name: String,     // display name
    pub cli_path: String, // resolved folder-opening CLI
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Workspace {
    pub id: String,           // built via `workspace_id()` (SOH-delimited) — do NOT concatenate the fields naively
    pub account_slug: String,
    pub account_name: String,
    pub ide_id: String,
    pub ide_name: String,
    pub project_path: String,
    pub mode: String,         // "seamless" | "wrapper"
    pub created: Option<u64>,
    pub last_opened: Option<u64>,
}

/// Deterministic id for a binding (dedupes re-adds of the same triple).
pub fn workspace_id(ide_id: &str, account_slug: &str, project_path: &str) -> String {
    format!("{ide_id}\u{1}{account_slug}\u{1}{project_path}")
}

/// Merge `terminal.integrated.env.osx.CLAUDE_CONFIG_DIR = <config_dir>` into an
/// existing `.vscode/settings.json`, preserving other keys. That single env var
/// is all a terminal needs: plain `claude` reads the profile's own login. Since
/// it's an env var (not PATH), it is immune to login-shell PATH re-ordering.
pub fn merge_vscode_settings(existing: &str, config_dir: &str) -> Result<String, String> {
    use serde_json::{Map, Value};
    let mut root: Value = if existing.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(existing).map_err(|e| format!("existing .vscode/settings.json is not valid JSON: {e}"))?
    };
    let obj = root.as_object_mut().ok_or("settings.json root is not an object")?;
    let env = obj.entry("terminal.integrated.env.osx".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let env = env.as_object_mut().ok_or("terminal.integrated.env.osx is not an object")?;
    env.insert("CLAUDE_CONFIG_DIR".to_string(), Value::String(config_dir.to_string()));
    serde_json::to_string_pretty(&root).map_err(|e| e.to_string())
}

/// Merge a `folderOpen` task into an existing `.vscode/tasks.json` (preserving any
/// other tasks). Upserts by `label`. `command` is the (ideally absolute) claude
/// binary, run as a process (not through a shell) with CLAUDE_CONFIG_DIR set in
/// its own env — so it resolves the right account regardless of the login
/// shell's PATH — and reveals a dedicated focused terminal.
pub fn merge_vscode_tasks(existing: &str, label: &str, claude_bin: &str, config_dir: &str) -> Result<String, String> {
    use serde_json::{json, Value};
    let mut root: Value = if existing.trim().is_empty() {
        json!({ "version": "2.0.0", "tasks": [] })
    } else {
        serde_json::from_str(existing).map_err(|e| format!("existing .vscode/tasks.json is not valid JSON: {e}"))?
    };
    let obj = root.as_object_mut().ok_or("tasks.json root is not an object")?;
    obj.entry("version".to_string()).or_insert_with(|| Value::String("2.0.0".into()));
    let tasks = obj.entry("tasks".to_string()).or_insert_with(|| Value::Array(vec![]));
    let arr = tasks.as_array_mut().ok_or("tasks.json 'tasks' is not an array")?;
    arr.retain(|t| t.get("label").and_then(|l| l.as_str()) != Some(label)); // upsert by label
    arr.push(json!({
        "label": label,
        "type": "process",
        "command": claude_bin,
        "options": { "env": { "CLAUDE_CONFIG_DIR": config_dir } },
        "presentation": { "reveal": "always", "panel": "dedicated", "focus": true, "clear": false },
        "runOptions": { "runOn": "folderOpen" },
        "problemMatcher": []
    }));
    serde_json::to_string_pretty(&root).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use crate::platform::home;
    use std::fs;
    use std::path::PathBuf;

    // Reuse the CLI account's config-dir root (per-slug dirs live here).
    pub fn cli_config_dir(slug: &str) -> PathBuf {
        home().join("Library/Application Support/ClaudeProfilesCLI").join(slug)
    }
    pub fn workspaces_file() -> PathBuf {
        home().join("Library/Application Support/ClaudeProfilesCLI/workspaces.json")
    }

    /// Candidate VS Code-family IDEs: (id, display, [bundle-relative CLI paths to probe]).
    fn candidates() -> Vec<(&'static str, &'static str, Vec<PathBuf>)> {
        let apps = |rel: &str| vec![
            PathBuf::from("/Applications").join(rel),
            home().join("Applications").join(rel),
        ];
        let mut out = vec![];
        out.push(("vscode", "Visual Studio Code",
            apps("Visual Studio Code.app/Contents/Resources/app/bin/code")));
        out.push(("cursor", "Cursor",
            apps("Cursor.app/Contents/Resources/app/bin/cursor")));
        out.push(("windsurf", "Windsurf", {
            let mut v = apps("Windsurf.app/Contents/Resources/app/bin/windsurf");
            v.push(home().join(".codeium/windsurf/bin/windsurf"));
            v
        }));
        out.push(("antigravity", "Antigravity IDE",
            apps("Antigravity IDE.app/Contents/Resources/app/bin/antigravity-ide")));
        out
    }

    pub fn detect_ides() -> Vec<Ide> {
        let mut ides = vec![];
        for (id, name, paths) in candidates() {
            if let Some(p) = paths.into_iter().find(|p| p.is_file()) {
                ides.push(Ide { id: id.into(), name: name.into(), cli_path: p.display().to_string() });
            }
        }
        ides
    }

    pub fn ide_cli(ide_id: &str) -> Option<String> {
        detect_ides().into_iter().find(|i| i.id == ide_id).map(|i| i.cli_path)
    }

    pub fn load_workspaces() -> Vec<Workspace> {
        match fs::read_to_string(workspaces_file()) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => vec![],
        }
    }
    pub fn save_workspaces(list: &[Workspace]) -> Result<(), String> {
        let f = workspaces_file();
        if let Some(d) = f.parent() { fs::create_dir_all(d).map_err(|e| e.to_string())?; }
        let s = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
        fs::write(&f, s).map_err(|e| e.to_string())
    }
}

// ---- public API (macOS) --------------------------------------------------
#[cfg(target_os = "macos")]
pub fn list_ides() -> Vec<Ide> { imp::detect_ides() }

/// Native folder picker (osascript). Returns None if the user cancels.
#[cfg(target_os = "macos")]
pub fn pick_folder() -> Result<Option<String>, String> {
    let (ok, out) = crate::platform::run(
        "osascript", &["-e", "POSIX path of (choose folder with prompt \"Select a project folder\")"]);
    let s = out.trim().to_string();
    if ok && !s.is_empty() { Ok(Some(s)) }
    else if s.contains("User canceled") || s.is_empty() { Ok(None) }
    else { Err(s) }
}

/// Open `project_path` in the given IDE with `account` active in its integrated
/// terminal: writes `.vscode/settings.json` (CLAUDE_CONFIG_DIR for manually
/// opened terminals) and `.vscode/tasks.json` (a folderOpen task that auto-opens
/// a Claude terminal on this account), then launches the IDE at the folder.
/// `_mode` is accepted for command/signature compatibility with callers
/// (workspaces store a `mode`) but no longer changes behavior — there is only
/// one mode now.
#[cfg(target_os = "macos")]
pub fn open_in_ide(account: &str, ide_id: &str, project_path: &str, _mode: &str) -> Result<(), String> {
    use crate::platform::slugify;
    let slug = slugify(account);
    if slug.is_empty() { return Err("Invalid account name.".into()); }
    let proj = std::path::PathBuf::from(project_path);
    if !proj.is_dir() { return Err(format!("Not a folder: {project_path}")); }
    let cli = imp::ide_cli(ide_id).ok_or_else(|| format!("IDE not found: {ide_id}"))?;
    let cfg = imp::cli_config_dir(&slug);
    let claude = crate::cli::imp_claude_bin_string();

    let vscode = proj.join(".vscode");
    std::fs::create_dir_all(&vscode).map_err(|e| e.to_string())?;
    let sp = vscode.join("settings.json");
    let merged = merge_vscode_settings(&std::fs::read_to_string(&sp).unwrap_or_default(), &cfg.display().to_string())?;
    std::fs::write(&sp, merged).map_err(|e| e.to_string())?;

    let tp = vscode.join("tasks.json");
    let label = format!("Claude Code — {account}");
    let tasks = merge_vscode_tasks(&std::fs::read_to_string(&tp).unwrap_or_default(), &label, &claude, &cfg.display().to_string())?;
    std::fs::write(&tp, tasks).map_err(|e| e.to_string())?;

    // Open the folder (VS Code-family CLIs reuse a running instance; workspace
    // settings still apply per-window).
    let (ok, e) = crate::platform::run(&cli, &[project_path]);
    if ok { Ok(()) } else { Err(e) }
}

#[cfg(target_os = "macos")]
pub fn list_workspaces() -> Vec<Workspace> {
    let mut w = imp::load_workspaces();
    w.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    w
}

#[cfg(target_os = "macos")]
pub fn save_workspace(account_slug: &str, account_name: &str, ide_id: &str, ide_name: &str,
                      project_path: &str, mode: &str, now: u64) -> Result<(), String> {
    let id = workspace_id(ide_id, account_slug, project_path);
    let mut list = imp::load_workspaces();
    if let Some(w) = list.iter_mut().find(|w| w.id == id) {
        w.last_opened = Some(now); w.mode = mode.to_string();
    } else {
        list.push(Workspace {
            id, account_slug: account_slug.into(), account_name: account_name.into(),
            ide_id: ide_id.into(), ide_name: ide_name.into(), project_path: project_path.into(),
            mode: mode.into(), created: Some(now), last_opened: Some(now),
        });
    }
    imp::save_workspaces(&list)
}

#[cfg(target_os = "macos")]
pub fn delete_workspace(id: &str) -> Result<(), String> {
    let mut list = imp::load_workspaces();
    list.retain(|w| w.id != id);
    imp::save_workspaces(&list)
}

/// Open a saved workspace by id (re-applies activation + launches), stamping
/// last_opened with `now`.
#[cfg(target_os = "macos")]
pub fn open_workspace(id: &str, now: u64) -> Result<(), String> {
    let list = imp::load_workspaces();
    let w = list.into_iter().find(|w| w.id == id).ok_or("No such workspace")?;
    open_in_ide(&w.account_name, &w.ide_id, &w.project_path, &w.mode)?;
    save_workspace(&w.account_slug, &w.account_name, &w.ide_id, &w.ide_name, &w.project_path, &w.mode, now)
}

// ---- non-macOS stubs -----------------------------------------------------
#[cfg(not(target_os = "macos"))]
const NOT_YET: &str = "IDE workspaces are macOS-only for now.";
#[cfg(not(target_os = "macos"))]
pub fn list_ides() -> Vec<Ide> { vec![] }
#[cfg(not(target_os = "macos"))]
pub fn pick_folder() -> Result<Option<String>, String> { Err(NOT_YET.into()) }
#[cfg(not(target_os = "macos"))]
pub fn open_in_ide(_a: &str, _i: &str, _p: &str, _m: &str) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(target_os = "macos"))]
pub fn list_workspaces() -> Vec<Workspace> { vec![] }
#[cfg(not(target_os = "macos"))]
pub fn save_workspace(_a: &str, _b: &str, _c: &str, _d: &str, _e: &str, _f: &str, _n: u64) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(target_os = "macos"))]
pub fn delete_workspace(_id: &str) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(target_os = "macos"))]
pub fn open_workspace(_id: &str, _n: u64) -> Result<(), String> { Err(NOT_YET.into()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_into_empty_creates_keys() {
        let out = merge_vscode_settings("", "/cfg/work").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let env = &v["terminal.integrated.env.osx"];
        assert_eq!(env["CLAUDE_CONFIG_DIR"], "/cfg/work");
    }

    #[test]
    fn merge_preserves_existing_user_settings() {
        let existing = r#"{"editor.fontSize":14,"terminal.integrated.env.osx":{"FOO":"bar"}}"#;
        let out = merge_vscode_settings(existing, "/c").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["editor.fontSize"], 14);                       // untouched
        assert_eq!(v["terminal.integrated.env.osx"]["FOO"], "bar"); // untouched
        assert_eq!(v["terminal.integrated.env.osx"]["CLAUDE_CONFIG_DIR"], "/c"); // added
    }

    #[test]
    fn merge_rejects_malformed_json() {
        assert!(merge_vscode_settings("{not json", "/c").is_err());
    }

    #[test]
    fn workspace_id_is_stable() {
        assert_eq!(workspace_id("cursor", "work", "/p"), workspace_id("cursor", "work", "/p"));
        assert_ne!(workspace_id("cursor", "work", "/p"), workspace_id("vscode", "work", "/p"));
    }

    #[test]
    fn tasks_merge_creates_folderopen_task() {
        let out = merge_vscode_tasks("", "Claude Code — Work", "/opt/homebrew/bin/claude", "/cfg/work").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["version"], "2.0.0");
        let t = &v["tasks"][0];
        assert_eq!(t["command"], "/opt/homebrew/bin/claude");
        assert_eq!(t["options"]["env"]["CLAUDE_CONFIG_DIR"], "/cfg/work");
        assert_eq!(t["runOptions"]["runOn"], "folderOpen");
        assert_eq!(t["type"], "process");
    }

    #[test]
    fn tasks_merge_upserts_and_preserves_others() {
        let existing = r#"{"version":"2.0.0","tasks":[{"label":"build","type":"shell","command":"make"},{"label":"Claude Code — Work","command":"/old"}]}"#;
        let out = merge_vscode_tasks(existing, "Claude Code — Work", "/new/claude", "/cfg/work").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let arr = v["tasks"].as_array().unwrap();
        assert_eq!(arr.len(), 2); // build preserved, our task upserted (not duplicated)
        assert!(arr.iter().any(|t| t["label"] == "build" && t["command"] == "make"));
        let ours: Vec<_> = arr.iter().filter(|t| t["label"] == "Claude Code — Work").collect();
        assert_eq!(ours.len(), 1);
        assert_eq!(ours[0]["command"], "/new/claude");
        assert_eq!(ours[0]["options"]["env"]["CLAUDE_CONFIG_DIR"], "/cfg/work");
    }

    #[test]
    fn tasks_merge_rejects_malformed() {
        assert!(merge_vscode_tasks("{bad", "L", "/c", "/cfg").is_err());
    }

    #[test]
    fn workspace_id_delimiter_avoids_collisions() {
        assert_ne!(workspace_id("ide", "wo", "rk/p"), workspace_id("ide", "work", "/p"));
    }

    #[test]
    fn merge_rejects_non_object_root() {
        assert!(merge_vscode_settings("[1,2,3]", "/c").is_err());
    }

    #[test]
    fn merge_rejects_non_object_env() {
        assert!(merge_vscode_settings(r#"{"terminal.integrated.env.osx":"oops"}"#, "/c").is_err());
    }
}
