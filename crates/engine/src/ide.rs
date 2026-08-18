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
    pub created: Option<u64>,
    pub last_opened: Option<u64>,
}

/// Deterministic id for a binding (dedupes re-adds of the same triple).
pub fn workspace_id(ide_id: &str, account_slug: &str, project_path: &str) -> String {
    format!("{ide_id}\u{1}{account_slug}\u{1}{project_path}")
}

/// Merge `<env_key>.CLAUDE_CONFIG_DIR = <config_dir>` into an existing
/// `.vscode/settings.json`, preserving other keys. VS Code keys the
/// integrated-terminal env block by OS, so `env_key` is the platform-specific
/// key (`terminal.integrated.env.osx` / `.windows` / `.linux`). That single env
/// var is all a terminal needs: plain `claude` reads the profile's own login.
/// Since it's an env var (not PATH), it is immune to login-shell PATH reordering.
pub fn merge_vscode_settings(existing: &str, config_dir: &str, env_key: &str) -> Result<String, String> {
    use serde_json::{Map, Value};
    let mut root: Value = if existing.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(existing).map_err(|e| format!("existing .vscode/settings.json is not valid JSON: {e}"))?
    };
    let obj = root.as_object_mut().ok_or("settings.json root is not an object")?;
    let env = obj.entry(env_key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let env = env.as_object_mut().ok_or_else(|| format!("{env_key} is not an object"))?;
    env.insert("CLAUDE_CONFIG_DIR".to_string(), Value::String(config_dir.to_string()));
    serde_json::to_string_pretty(&root).map_err(|e| e.to_string())
}

/// Merge a `folderOpen` task into an existing `.vscode/tasks.json` (preserving any
/// other tasks). Upserts by `label`. `command` is the (ideally absolute) claude
/// binary, run with CLAUDE_CONFIG_DIR set in its own env — so it resolves the
/// right account regardless of the login shell's PATH — and reveals a dedicated
/// focused terminal. `task_type` is the VS Code task type: `"process"` (macOS,
/// spawns the binary directly) or `"shell"` (Windows, where a `.cmd` shim
/// launches more reliably through a shell).
pub fn merge_vscode_tasks(existing: &str, label: &str, command: &str, config_dir: &str, task_type: &str) -> Result<String, String> {
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
        "type": task_type,
        "command": command,
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
    // MUST byte-match cli::imp::config_dir.
    pub fn cli_config_dir(slug: &str) -> PathBuf {
        home().join("Library/Application Support/Personae/CLI").join(slug)
    }
    pub fn workspaces_file() -> PathBuf {
        home().join("Library/Application Support/Personae/CLI/workspaces.json")
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
        // Defensive: `cli::list()` also runs this, but `list_workspaces` reads
        // the same migrated root and could be invoked first on startup.
        crate::cli::migrate_legacy_dirs();
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

// Windows IDE engine — WRITTEN FROM WINDOWS DOCS + THE VS Code-family install
// layouts, NOT YET RUN ON REAL WINDOWS. Isolation is by CLAUDE_CONFIG_DIR alone:
// `cli_config_dir` MUST byte-match `cli::imp_win::config_dir` so IDE terminals
// and CLI launcher terminals resolve the same account. Every process / dialog /
// quoting assumption is marked TODO(verify); see docs/WINDOWS-TEST-PLAN.md.
#[cfg(windows)]
mod imp_win {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn appdata() -> PathBuf { PathBuf::from(std::env::var("APPDATA").unwrap_or_default()) }
    fn localappdata() -> PathBuf { PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()) }
    fn programfiles() -> PathBuf { PathBuf::from(std::env::var("ProgramFiles").unwrap_or_default()) }

    // MUST byte-match cli::imp_win::config_dir: %APPDATA%\Personae\CLI\<slug>
    pub fn cli_config_dir(slug: &str) -> PathBuf {
        appdata().join("Personae").join("CLI").join(slug)
    }
    // %APPDATA%\Personae\CLI\workspaces.json
    pub fn workspaces_file() -> PathBuf {
        appdata().join("Personae").join("CLI").join("workspaces.json")
    }

    /// Like `crate::platform::run`, but with CREATE_NO_WINDOW so the IDE-discovery
    /// `where` probe and the folder-picker `powershell` call don't flash a console
    /// window. Kept local to ide.rs so the shared `platform::run` stays untouched
    /// (this branch's `platform.rs` doesn't set CREATE_NO_WINDOW yet).
    pub fn run_hidden(program: &str, args: &[&str]) -> (bool, String) {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new(program);
        cmd.args(args).creation_flags(0x08000000); // CREATE_NO_WINDOW
        match cmd.output() {
            Ok(o) => {
                let mut s = String::from_utf8_lossy(&o.stdout).to_string();
                s.push_str(&String::from_utf8_lossy(&o.stderr));
                (o.status.success(), s)
            }
            Err(e) => (false, format!("{e}")),
        }
    }

    /// Candidate VS Code-family IDEs: (id, display, PATH cli name for `where`,
    /// [absolute launcher paths to probe]). First existing `.cmd`/`.exe` wins.
    fn candidates() -> Vec<(&'static str, &'static str, &'static str, Vec<PathBuf>)> {
        let lad = localappdata();
        let pf = programfiles();
        vec![
            ("vscode", "Visual Studio Code", "code", vec![
                lad.join(r"Programs\Microsoft VS Code\bin\code.cmd"),
                pf.join(r"Microsoft VS Code\bin\code.cmd"),
            ]),
            ("cursor", "Cursor", "cursor", vec![
                lad.join(r"Programs\cursor\resources\app\bin\cursor.cmd"),
            ]),
            ("windsurf", "Windsurf", "windsurf", vec![
                lad.join(r"Programs\Windsurf\bin\windsurf.cmd"),
            ]),
            // TODO(verify): Antigravity's Windows install layout + CLI name are
            // unconfirmed; probe the likely Electron `resources\app\bin` and `bin`
            // shims under %LOCALAPPDATA%\Programs\.
            ("antigravity", "Antigravity IDE", "antigravity", vec![
                lad.join(r"Programs\Antigravity\resources\app\bin\antigravity.cmd"),
                lad.join(r"Programs\Antigravity\bin\antigravity.cmd"),
            ]),
        ]
    }

    pub fn detect_ides() -> Vec<Ide> {
        let mut ides = vec![];
        for (id, name, cli, paths) in candidates() {
            // TODO(verify): `where <cli>` resolves the IDE's `.cmd` shim on PATH.
            let mut resolved: Option<String> = None;
            let (ok, out) = run_hidden("where", &[cli]);
            if ok {
                for line in out.lines() {
                    let pb = PathBuf::from(line.trim());
                    if pb.is_file() { resolved = Some(pb.display().to_string()); break; }
                }
            }
            // Then the candidate paths — first existing `.cmd`/`.exe` wins.
            if resolved.is_none() {
                if let Some(p) = paths.into_iter().find(|p| p.is_file()) {
                    resolved = Some(p.display().to_string());
                }
            }
            if let Some(cli_path) = resolved {
                ides.push(Ide { id: id.into(), name: name.into(), cli_path });
            }
        }
        ides
    }

    pub fn ide_cli(ide_id: &str) -> Option<String> {
        detect_ides().into_iter().find(|i| i.id == ide_id).map(|i| i.cli_path)
    }

    pub fn load_workspaces() -> Vec<Workspace> {
        // Defensive: `cli::list()` also runs this, but `list_workspaces` reads
        // the same migrated root and could be invoked first on startup.
        crate::cli::migrate_legacy_dirs();
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
    if ok && !s.is_empty() {
        // `choose folder` returns a trailing slash — strip it so the path, the
        // workspace id, and the folder-name label are clean (keep "/" for root).
        let c = s.trim_end_matches('/');
        Ok(Some(if c.is_empty() { "/".into() } else { c.to_string() }))
    }
    else if s.contains("User canceled") || s.is_empty() { Ok(None) }
    else { Err(s) }
}

/// Open `project_path` in the given IDE with `account` active in its integrated
/// terminal: writes `.vscode/settings.json` (CLAUDE_CONFIG_DIR for manually
/// opened terminals) and `.vscode/tasks.json` (a folderOpen task that auto-opens
/// a Claude terminal on this account), then launches the IDE at the folder.
#[cfg(target_os = "macos")]
pub fn open_in_ide(account: &str, ide_id: &str, project_path: &str) -> Result<(), String> {
    use crate::platform::slugify;
    let slug = slugify(account);
    if slug.is_empty() { return Err("Invalid account name.".into()); }
    let proj = std::path::PathBuf::from(project_path);
    if !proj.is_dir() { return Err(format!("Not a folder: {project_path}")); }
    let cli = imp::ide_cli(ide_id).ok_or_else(|| format!("IDE not found: {ide_id}"))?;
    let cfg = imp::cli_config_dir(&slug);
    let claude = crate::cli::imp_claude_bin_string();
    // If already logged in, skip the first-run menu so the IDE terminal goes
    // straight in on the account (auth login doesn't set this flag itself).
    crate::cli::ensure_onboarded(account);

    let vscode = proj.join(".vscode");
    std::fs::create_dir_all(&vscode).map_err(|e| e.to_string())?;
    let sp = vscode.join("settings.json");
    let merged = merge_vscode_settings(&std::fs::read_to_string(&sp).unwrap_or_default(), &cfg.display().to_string(), "terminal.integrated.env.osx")?;
    std::fs::write(&sp, merged).map_err(|e| e.to_string())?;

    let tp = vscode.join("tasks.json");
    let label = format!("Claude Code — {account}");
    let tasks = merge_vscode_tasks(&std::fs::read_to_string(&tp).unwrap_or_default(), &label, &claude, &cfg.display().to_string(), "process")?;
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
                      project_path: &str, now: u64) -> Result<(), String> {
    let id = workspace_id(ide_id, account_slug, project_path);
    let mut list = imp::load_workspaces();
    if let Some(w) = list.iter_mut().find(|w| w.id == id) {
        w.last_opened = Some(now);
    } else {
        list.push(Workspace {
            id, account_slug: account_slug.into(), account_name: account_name.into(),
            ide_id: ide_id.into(), ide_name: ide_name.into(), project_path: project_path.into(),
            created: Some(now), last_opened: Some(now),
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
    open_in_ide(&w.account_name, &w.ide_id, &w.project_path)?;
    save_workspace(&w.account_slug, &w.account_name, &w.ide_id, &w.ide_name, &w.project_path, now)
}

// ---- public API (Windows) ------------------------------------------------
// Isolation is by CLAUDE_CONFIG_DIR alone (mirrors macOS). Every process /
// dialog / quoting assumption below is marked TODO(verify) for the real-Windows
// pass; see docs/WINDOWS-TEST-PLAN.md.
#[cfg(windows)]
pub fn list_ides() -> Vec<Ide> { imp_win::detect_ides() }

/// Native folder picker via PowerShell's WinForms FolderBrowserDialog. Returns
/// None if the user cancels (the dialog prints nothing on Cancel). Runs under
/// `pwsh` when available so the dialog is the modern Explorer-style picker,
/// not Windows PowerShell 5.1's legacy tree view — see
/// `platform::run_powershell_hidden`'s doc comment.
#[cfg(windows)]
pub fn pick_folder() -> Result<Option<String>, String> {
    // -STA is required for WinForms dialogs. The script loads System.Windows.Forms,
    // shows the dialog, and prints SelectedPath only on OK (Cancel => no output).
    // TODO(verify): the dialog appears, OK prints the chosen path, and Cancel
    // yields empty output (so cancel => Ok(None), never an error).
    let script = "Add-Type -AssemblyName System.Windows.Forms; \
                  $f=New-Object System.Windows.Forms.FolderBrowserDialog; \
                  if($f.ShowDialog() -eq 'OK'){Write-Output $f.SelectedPath}";
    let (ok, out) = crate::platform::run_powershell_hidden(
        &["-STA", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
    );
    // A non-zero exit is a genuine failure — don't let it masquerade as a cancel.
    if !ok { return Err(if out.trim().is_empty() { "Folder picker failed to run.".into() } else { out.trim().to_string() }); }
    // Exit 0: Cancel writes nothing (=> None); OK prints the chosen path.
    let s = out.trim();
    if s.is_empty() { Ok(None) } else { Ok(Some(s.to_string())) }
}

/// Open `project_path` in the given IDE with `account` active in its integrated
/// terminal: writes `.vscode/settings.json` (CLAUDE_CONFIG_DIR under the Windows
/// terminal-env key) and `.vscode/tasks.json` (a folderOpen SHELL task that
/// auto-opens a Claude terminal on this account), then launches the IDE at the
/// folder.
#[cfg(windows)]
pub fn open_in_ide(account: &str, ide_id: &str, project_path: &str) -> Result<(), String> {
    use crate::platform::slugify;
    let slug = slugify(account);
    if slug.is_empty() { return Err("Invalid account name.".into()); }
    let proj = std::path::PathBuf::from(project_path);
    if !proj.is_dir() { return Err(format!("Not a folder: {project_path}")); }
    let cli = imp_win::ide_cli(ide_id).ok_or_else(|| format!("IDE not found: {ide_id}"))?;
    let cfg = imp_win::cli_config_dir(&slug);
    let claude = crate::cli::imp_claude_bin_string();
    // If already logged in, skip the first-run menu so the IDE terminal goes
    // straight in on the account (auth login doesn't set this flag itself).
    crate::cli::ensure_onboarded(account);

    let vscode = proj.join(".vscode");
    std::fs::create_dir_all(&vscode).map_err(|e| e.to_string())?;
    let sp = vscode.join("settings.json");
    let merged = merge_vscode_settings(&std::fs::read_to_string(&sp).unwrap_or_default(), &cfg.display().to_string(), "terminal.integrated.env.windows")?;
    std::fs::write(&sp, merged).map_err(|e| e.to_string())?;

    let tp = vscode.join("tasks.json");
    let label = format!("Claude Code — {account}");
    // A `.cmd` shim launches more reliably as a "shell" task than a "process".
    let tasks = merge_vscode_tasks(&std::fs::read_to_string(&tp).unwrap_or_default(), &label, &claude, &cfg.display().to_string(), "shell")?;
    std::fs::write(&tp, tasks).map_err(|e| e.to_string())?;

    // A `.cmd` cannot be spawned directly by std::process::Command, so open the
    // folder through `cmd /C`. Both the IDE shim (the DEFAULT VS Code install is
    // "C:\Program Files\Microsoft VS Code\bin\code.cmd") and the project path can
    // contain spaces; letting std apply its per-arg MSVC quoting collides with
    // cmd's "strip the first and last quote" rule and mangles the command. So we
    // build the command line by hand with cmd's outer-quote idiom
    // — `cmd /C ""<exe>" "<arg>""` (the outer pair makes cmd run the middle
    // verbatim) — and pass it via raw_arg. CREATE_NO_WINDOW keeps this spawn from
    // flashing a console. TODO(verify): confirm the outer-quote idiom on a real
    // Windows box with a spaced project path under the default VS Code install.
    use std::os::windows::process::CommandExt;
    let line = format!("\"\"{}\" \"{}\"\"", cli, project_path);
    let status = std::process::Command::new("cmd")
        .raw_arg("/C")
        .raw_arg(&line)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("IDE launch failed (exit {status})")) }
}

#[cfg(windows)]
pub fn list_workspaces() -> Vec<Workspace> {
    let mut w = imp_win::load_workspaces();
    w.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    w
}

#[cfg(windows)]
pub fn save_workspace(account_slug: &str, account_name: &str, ide_id: &str, ide_name: &str,
                      project_path: &str, now: u64) -> Result<(), String> {
    let id = workspace_id(ide_id, account_slug, project_path);
    let mut list = imp_win::load_workspaces();
    if let Some(w) = list.iter_mut().find(|w| w.id == id) {
        w.last_opened = Some(now);
    } else {
        list.push(Workspace {
            id, account_slug: account_slug.into(), account_name: account_name.into(),
            ide_id: ide_id.into(), ide_name: ide_name.into(), project_path: project_path.into(),
            created: Some(now), last_opened: Some(now),
        });
    }
    imp_win::save_workspaces(&list)
}

#[cfg(windows)]
pub fn delete_workspace(id: &str) -> Result<(), String> {
    let mut list = imp_win::load_workspaces();
    list.retain(|w| w.id != id);
    imp_win::save_workspaces(&list)
}

/// Open a saved workspace by id (re-applies activation + launches), stamping
/// last_opened with `now`.
#[cfg(windows)]
pub fn open_workspace(id: &str, now: u64) -> Result<(), String> {
    let list = imp_win::load_workspaces();
    let w = list.into_iter().find(|w| w.id == id).ok_or("No such workspace")?;
    open_in_ide(&w.account_name, &w.ide_id, &w.project_path)?;
    save_workspace(&w.account_slug, &w.account_name, &w.ide_id, &w.ide_name, &w.project_path, now)
}

// ---- non-macOS/Windows stubs ---------------------------------------------
#[cfg(not(any(target_os = "macos", windows)))]
const NOT_YET: &str = "IDE workspaces are macOS/Windows-only for now.";
#[cfg(not(any(target_os = "macos", windows)))]
pub fn list_ides() -> Vec<Ide> { vec![] }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn pick_folder() -> Result<Option<String>, String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn open_in_ide(_a: &str, _i: &str, _p: &str) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn list_workspaces() -> Vec<Workspace> { vec![] }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn save_workspace(_a: &str, _b: &str, _c: &str, _d: &str, _e: &str, _n: u64) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn delete_workspace(_id: &str) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn open_workspace(_id: &str, _n: u64) -> Result<(), String> { Err(NOT_YET.into()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_into_empty_creates_keys() {
        let out = merge_vscode_settings("", "/cfg/work", "terminal.integrated.env.osx").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let env = &v["terminal.integrated.env.osx"];
        assert_eq!(env["CLAUDE_CONFIG_DIR"], "/cfg/work");
    }

    #[test]
    fn merge_preserves_existing_user_settings() {
        let existing = r#"{"editor.fontSize":14,"terminal.integrated.env.osx":{"FOO":"bar"}}"#;
        let out = merge_vscode_settings(existing, "/c", "terminal.integrated.env.osx").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["editor.fontSize"], 14);                       // untouched
        assert_eq!(v["terminal.integrated.env.osx"]["FOO"], "bar"); // untouched
        assert_eq!(v["terminal.integrated.env.osx"]["CLAUDE_CONFIG_DIR"], "/c"); // added
    }

    #[test]
    fn merge_rejects_malformed_json() {
        assert!(merge_vscode_settings("{not json", "/c", "terminal.integrated.env.osx").is_err());
    }

    #[test]
    fn merge_settings_windows_env_key_nests_under_windows() {
        let out = merge_vscode_settings("", r"C:\cfg", "terminal.integrated.env.windows").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        // The env block lives under the Windows key, not the osx key.
        assert_eq!(v["terminal.integrated.env.windows"]["CLAUDE_CONFIG_DIR"], r"C:\cfg");
        assert!(v.get("terminal.integrated.env.osx").is_none());
    }

    #[test]
    fn workspace_id_is_stable() {
        assert_eq!(workspace_id("cursor", "work", "/p"), workspace_id("cursor", "work", "/p"));
        assert_ne!(workspace_id("cursor", "work", "/p"), workspace_id("vscode", "work", "/p"));
    }

    #[test]
    fn tasks_merge_creates_folderopen_task() {
        let out = merge_vscode_tasks("", "Claude Code — Work", "/opt/homebrew/bin/claude", "/cfg/work", "process").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["version"], "2.0.0");
        let t = &v["tasks"][0];
        assert_eq!(t["command"], "/opt/homebrew/bin/claude");
        assert_eq!(t["options"]["env"]["CLAUDE_CONFIG_DIR"], "/cfg/work");
        assert_eq!(t["runOptions"]["runOn"], "folderOpen");
        assert_eq!(t["type"], "process");
    }

    #[test]
    fn tasks_merge_shell_type_yields_shell() {
        // Windows launches the `.cmd` shim more reliably as a shell task.
        let out = merge_vscode_tasks("", "Claude Code — Work", "claude", r"C:\cfg\work", "shell").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let t = &v["tasks"][0];
        assert_eq!(t["type"], "shell");
        assert_eq!(t["command"], "claude");
        assert_eq!(t["options"]["env"]["CLAUDE_CONFIG_DIR"], r"C:\cfg\work");
    }

    #[test]
    fn tasks_merge_upserts_and_preserves_others() {
        let existing = r#"{"version":"2.0.0","tasks":[{"label":"build","type":"shell","command":"make"},{"label":"Claude Code — Work","command":"/old"}]}"#;
        let out = merge_vscode_tasks(existing, "Claude Code — Work", "/new/claude", "/cfg/work", "process").unwrap();
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
        assert!(merge_vscode_tasks("{bad", "L", "/c", "/cfg", "process").is_err());
    }

    #[test]
    fn workspace_id_delimiter_avoids_collisions() {
        assert_ne!(workspace_id("ide", "wo", "rk/p"), workspace_id("ide", "work", "/p"));
    }

    #[test]
    fn merge_rejects_non_object_root() {
        assert!(merge_vscode_settings("[1,2,3]", "/c", "terminal.integrated.env.osx").is_err());
    }

    #[test]
    fn merge_rejects_non_object_env() {
        assert!(merge_vscode_settings(r#"{"terminal.integrated.env.osx":"oops"}"#, "/c", "terminal.integrated.env.osx").is_err());
    }
}
