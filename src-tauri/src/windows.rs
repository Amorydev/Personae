// Windows implementation — WRITTEN FROM CLAUDE'S CODE + WINDOWS DOCS, NOT YET
// TESTED ON A REAL WINDOWS MACHINE. Every point marked TODO(verify) must be
// confirmed on Windows; see docs/WINDOWS-TEST-PLAN.md for the checklist.
//
// What is solid (from docs): data isolation via CLAUDE_USER_DATA_DIR +
// --user-data-dir; Squirrel path %LOCALAPPDATA%\AnthropicClaude\app-<ver>\Claude.exe;
// newer MSIX alias %LOCALAPPDATA%\Microsoft\WindowsApps\Claude.exe.
//
// What is genuinely uncertain: whether per-profile taskbar SEPARATION is possible
// at all. Claude (Electron) sets its own process AUMID at launch, which Windows
// lets override the shortcut's AUMID for grouping. We set the .lnk AUMID anyway
// (harmless, future-proof) but do NOT promise separate taskbar groups.
#![cfg(windows)]
use crate::platform::*;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static CLAUDE_EXE_CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();

// Bundled PowerShell helpers (kept as inspectable files under scripts/win/,
// embedded into the binary, and written out next to the profiles at runtime).
const PS_SET_AUMID: &str = include_str!("../scripts/win/set-aumid.ps1");
const PS_TINT_ICO: &str = include_str!("../scripts/win/tint-ico.ps1");

pub struct Win;

impl Win {
    pub fn new() -> Self {
        Win
    }

    fn appdata(&self) -> PathBuf {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_default())
    }
    fn localappdata(&self) -> PathBuf {
        PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default())
    }

    /// Per-profile launchers (`<slug>.cmd`), icons (`<slug>.ico`), display-name
    /// sidecars (`<slug>.name`), and the emitted PS helpers (`_helpers\`).
    fn app_root(&self) -> PathBuf {
        self.localappdata().join("ClaudeProfiles").join("apps")
    }
    /// Isolated Claude data lives under Roaming, one dir per slug.
    fn data_root(&self) -> PathBuf {
        self.appdata().join("ClaudeProfiles")
    }
    fn start_menu(&self) -> PathBuf {
        self.appdata()
            .join(r"Microsoft\Windows\Start Menu\Programs\Claude Profiles")
    }

    fn launcher_cmd(&self, slug: &str) -> PathBuf {
        self.app_root().join(format!("{slug}.cmd"))
    }
    fn ico_file(&self, slug: &str) -> PathBuf {
        self.app_root().join(format!("{slug}.ico"))
    }
    fn name_file(&self, slug: &str) -> PathBuf {
        self.app_root().join(format!("{slug}.name"))
    }
    fn tint_file(&self, slug: &str) -> PathBuf {
        self.data_root().join(slug).join(".claude-profile-tint")
    }
    fn lnk_path(&self, name: &str) -> PathBuf {
        self.start_menu().join(format!("{name}.lnk"))
    }

    /// Root folders Claude may be installed under, across installer types and
    /// per-user vs per-machine installs. Each is probed for a versionless stub
    /// and for the newest `app-<ver>` (Squirrel) subfolder.
    fn candidate_roots(&self) -> Vec<PathBuf> {
        let lad = self.localappdata();
        let mut roots = vec![
            lad.join("AnthropicClaude"),          // classic Squirrel (per-user)
            lad.join(r"Programs\AnthropicClaude"), // newer per-user Programs layout
            lad.join(r"Programs\Claude"),
            lad.join("Claude"),
        ];
        // Per-machine installs under Program Files (both bitnesses).
        for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
            if let Ok(pf) = std::env::var(var) {
                let pf = PathBuf::from(pf);
                roots.push(pf.join("AnthropicClaude"));
                roots.push(pf.join("Claude"));
            }
        }
        roots
    }

    /// Find `Claude.exe` inside one root: prefer the versionless stub (survives
    /// updates), else the highest-numbered `app-<ver>` (so 0.10 > 0.9).
    fn exe_in_root(root: &PathBuf) -> Option<PathBuf> {
        let stub = root.join("Claude.exe");
        if stub.is_file() {
            return Some(stub);
        }
        let mut best: Option<(Vec<u32>, PathBuf)> = None;
        if let Ok(rd) = fs::read_dir(root) {
            for e in rd.flatten() {
                let p = e.path();
                let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if let Some(ver) = fname.strip_prefix("app-") {
                    let exe = p.join("Claude.exe");
                    if exe.is_file() {
                        let parsed: Vec<u32> =
                            ver.split('.').filter_map(|x| x.parse().ok()).collect();
                        let take = match &best {
                            Some((bv, _)) => parsed > *bv,
                            None => true,
                        };
                        if take {
                            best = Some((parsed, exe));
                        }
                    }
                }
            }
        }
        best.map(|(_, exe)| exe)
    }

    /// Last-resort: ask Windows where Claude.exe is via the registry. The
    /// standard "App Paths" key holds the full exe path installers register;
    /// Uninstall\*.DisplayIcon is a secondary source. Read via `reg.exe` (no
    /// extra crate); only reached when the filesystem probe finds nothing.
    fn claude_from_registry() -> Option<PathBuf> {
        let queries: [(&str, &str); 4] = [
            (r"HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths\Claude.exe", "/ve"),
            (r"HKLM\Software\Microsoft\Windows\CurrentVersion\App Paths\Claude.exe", "/ve"),
            (r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Claude", "DisplayIcon"),
            (r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\AnthropicClaude", "DisplayIcon"),
        ];
        for (key, val) in queries {
            let args: Vec<&str> = if val == "/ve" {
                vec!["query", key, "/ve"]
            } else {
                vec!["query", key, "/v", val]
            };
            let (_, out) = run("reg", &args);
            if let Some(pb) = Self::parse_reg_path(&out) {
                return Some(pb);
            }
        }
        None
    }

    /// Pull an existing `...\Claude.exe` out of `reg query` output. Handles paths
    /// with spaces (parses after the `REG_SZ` type token) and a trailing `,<idx>`
    /// icon index on DisplayIcon values.
    fn parse_reg_path(out: &str) -> Option<PathBuf> {
        for line in out.lines() {
            if let Some(idx) = line.find("REG_SZ") {
                let val = line[idx + "REG_SZ".len()..].trim();
                // Strip a trailing icon index like ",0" (present on DisplayIcon).
                let val = if val.to_ascii_lowercase().ends_with(".exe") {
                    val
                } else {
                    val.rsplit_once(',').map(|(p, _)| p).unwrap_or(val).trim()
                };
                if val.to_ascii_lowercase().ends_with("claude.exe") {
                    let pb = PathBuf::from(val);
                    if pb.is_file() {
                        return Some(pb);
                    }
                }
            }
        }
        None
    }

    /// Check running `Claude.exe` processes for executable path via PowerShell.
    fn claude_from_running_process() -> Option<PathBuf> {
        let ps = "(Get-Process Claude -ErrorAction SilentlyContinue).Path";
        let (ok, out) = run(
            "powershell",
            &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps],
        );
        if ok {
            for line in out.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let pb = PathBuf::from(trimmed);
                    if pb.is_file() {
                        return Some(pb);
                    }
                }
            }
        }
        None
    }

    /// Check Windows Store / AppX / MSIX package location via PowerShell.
    fn claude_from_appx() -> Option<PathBuf> {
        let ps = "(Get-AppxPackage *Claude* -ErrorAction SilentlyContinue).InstallLocation";
        let (ok, out) = run(
            "powershell",
            &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps],
        );
        if ok {
            for line in out.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let base = PathBuf::from(trimmed);
                    let app_exe = base.join("app").join("Claude.exe");
                    if app_exe.is_file() {
                        return Some(app_exe);
                    }
                    let root_exe = base.join("Claude.exe");
                    if root_exe.is_file() {
                        return Some(root_exe);
                    }
                }
            }
        }
        None
    }

    /// Locate Claude.exe. Cached in memory using OnceLock so subsequent calls take 0ms.
    fn claude_exe(&self) -> Option<PathBuf> {
        CLAUDE_EXE_CACHE
            .get_or_init(|| self.resolve_claude_exe())
            .clone()
    }

    fn resolve_claude_exe(&self) -> Option<PathBuf> {
        if let Ok(p) = std::env::var("CLAUDE_APP") {
            let pb = PathBuf::from(p);
            if pb.is_file() {
                return Some(pb);
            }
        }
        let msix = self.localappdata().join(r"Microsoft\WindowsApps\Claude.exe");
        if msix.is_file() {
            return Some(msix);
        }
        if let Some(exe) = Self::claude_from_running_process() {
            return Some(exe);
        }
        if let Some(exe) = Self::claude_from_appx() {
            return Some(exe);
        }
        for root in self.candidate_roots() {
            if let Some(exe) = Self::exe_in_root(&root) {
                return Some(exe);
            }
        }
        Self::claude_from_registry()
    }

    /// Write the two PS helpers next to the profiles; return their paths.
    fn ensure_helpers(&self) -> Result<(PathBuf, PathBuf), String> {
        let dir = self.app_root().join("_helpers");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let a = dir.join("set-aumid.ps1");
        let t = dir.join("tint-ico.ps1");
        fs::write(&a, PS_SET_AUMID).map_err(|e| e.to_string())?;
        fs::write(&t, PS_TINT_ICO).map_err(|e| e.to_string())?;
        Ok((a, t))
    }

    /// Invoke a bundled .ps1 with `-Key Value` params (Bypass so unsigned runs).
    fn run_ps(&self, script: &PathBuf, params: &[(&str, &str)]) -> bool {
        let mut args: Vec<String> = vec![
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script.display().to_string(),
        ];
        for (k, v) in params {
            args.push(format!("-{k}"));
            args.push((*v).to_string());
        }
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run("powershell", &refs).0
    }

    fn write_launcher(&self, slug: &str, data_dir: &PathBuf, exe: &PathBuf) -> Result<(), String> {
        // A .cmd that isolates data then launches Claude. We set BOTH:
        //   * CLAUDE_USER_DATA_DIR (Claude's own relocation env var), AND
        //   * --user-data-dir=<dir> on the command line.
        // The flag is what makes running-detection possible: Win32_Process exposes
        // CommandLine but NOT the environment, so an env-only launch was invisible
        // to profile_pids(). Both point at the same dir (no conflict).
        let dd = data_dir.display().to_string();
        let body = format!(
            "@echo off\r\nset \"CLAUDE_USER_DATA_DIR={dd}\"\r\nstart \"\" \"{exe}\" \"--user-data-dir={dd}\" %*\r\n",
            dd = dd,
            exe = exe.display()
        );
        fs::write(self.launcher_cmd(slug), body).map_err(|e| e.to_string())
    }

    fn count_apps(&self) -> usize {
        fs::read_dir(self.app_root())
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().extension().map_or(false, |x| x == "cmd"))
                    .count()
            })
            .unwrap_or(0)
    }

    /// Query all running `Claude.exe` process CommandLines in a single PowerShell call.
    fn all_running_commandlines(&self) -> Vec<String> {
        let ps = "Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'Claude.exe' } | Select-Object -ExpandProperty CommandLine";
        let (ok, out) = run(
            "powershell",
            &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps],
        );
        if ok {
            out.lines().map(|s| s.to_string()).collect()
        } else {
            vec![]
        }
    }

    /// pids of Claude processes whose CommandLine references this data dir.
    /// Works because write_launcher passes --user-data-dir=<dir> (see there).
    fn profile_pids(&self, data_dir: &PathBuf) -> Vec<String> {
        // Backslashes are LITERAL to PowerShell -like (only * ? [ ] are wildcards),
        // so do NOT double them; escape single-quotes for the PS single-quoted string.
        let dd = data_dir.display().to_string().replace('\'', "''");
        let ps = format!(
            "Get-CimInstance Win32_Process | Where-Object {{ $_.Name -eq 'Claude.exe' -and $_.CommandLine -like '*{dd}*' }} | Select-Object -ExpandProperty ProcessId",
            dd = dd
        );
        let (_, out) = run(
            "powershell",
            &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps],
        );
        out.split_whitespace().map(|s| s.to_string()).collect()
    }
}

fn ps_quote(s: &str) -> String {
    s.replace('\'', "''")
}

impl Platform for Win {
    fn claude_found(&self) -> bool {
        self.claude_exe().is_some()
    }

    fn create(&self, name: &str, color: Option<String>, _isolation: &str) -> Result<(), String> {
        let exe = self
            .claude_exe()
            .ok_or("Claude Desktop not found (set CLAUDE_APP).")?;
        let slug = slugify(name);
        if slug.is_empty() {
            return Err("Name must contain letters or numbers.".into());
        }
        let data_dir = self.data_root().join(&slug);
        if self.launcher_cmd(&slug).exists() {
            return Err(format!("Profile already exists: {name}"));
        }

        let pre_count = self.count_apps();
        fs::create_dir_all(self.app_root()).map_err(|e| e.to_string())?;
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

        self.write_launcher(&slug, &data_dir, &exe)?;
        // The .cmd is named by slug; remember the original display name.
        let _ = fs::write(self.name_file(&slug), name);

        let hex = match color {
            Some(c) => resolve_color(&c).ok_or(format!("Unrecognized color '{c}'."))?,
            None => PALETTE[pre_count % PALETTE.len()].to_string(),
        };
        let _ = fs::write(self.tint_file(&slug), format!("#{hex}"));

        let (aumid_ps, tint_ps) = self.ensure_helpers().unwrap_or_default();
        let ico = self.ico_file(&slug);
        let lnk = self.lnk_path(name);
        let cmd = self.launcher_cmd(&slug);
        let app_root = self.app_root();

        // Consolidated PowerShell setup script: tint icon, create shortcut, set AUMID in ONE invocation.
        let ps_script = format!(
            "$ErrorActionPreference='SilentlyContinue'; \
             if ('{tint_ps}' -ne '' -and (Test-Path '{tint_ps}')) {{ & '{tint_ps}' -Exe '{exe}' -Out '{ico}' -Hex '{hex}' }}; \
             $iconLoc = if (Test-Path '{ico}') {{ '{ico},0' }} else {{ '{exe},0' }}; \
             $w = New-Object -ComObject WScript.Shell; \
             $s = $w.CreateShortcut('{lnk}'); \
             $s.TargetPath = '{cmd}'; \
             $s.WorkingDirectory = '{wd}'; \
             $s.IconLocation = $iconLoc; \
             $s.WindowStyle = 7; \
             $s.Save(); \
             if ('{aumid_ps}' -ne '' -and (Test-Path '{aumid_ps}')) {{ & '{aumid_ps}' -Lnk '{lnk}' -Aumid '{aumid}' }}",
            tint_ps = ps_quote(&tint_ps.display().to_string()),
            exe = ps_quote(&exe.display().to_string()),
            ico = ps_quote(&ico.display().to_string()),
            hex = hex,
            lnk = ps_quote(&lnk.display().to_string()),
            cmd = ps_quote(&cmd.display().to_string()),
            wd = ps_quote(&app_root.display().to_string()),
            aumid_ps = ps_quote(&aumid_ps.display().to_string()),
            aumid = format!("{BUNDLE_PREFIX}.{slug}"),
        );

        let _ = run(
            "powershell",
            &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script],
        );
        Ok(())
    }

    fn list(&self) -> Vec<Profile> {
        let mut out = vec![];
        let rd = match fs::read_dir(self.app_root()) {
            Ok(r) => r,
            Err(_) => return out,
        };
        let entries: Vec<_> = rd
            .flatten()
            .filter(|e| e.path().extension().map_or(false, |x| x == "cmd"))
            .collect();
        if entries.is_empty() {
            return out;
        }

        // Fetch running processes ONCE for all profiles instead of N times.
        let running_cmds = self.all_running_commandlines();

        for e in entries {
            let p = e.path();
            let slug = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let name = fs::read_to_string(self.name_file(&slug))
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| slug.clone());
            let data_dir = self.data_root().join(&slug);
            let dd_lower = data_dir.display().to_string().to_lowercase();
            let running = running_cmds.iter().any(|cmd| cmd.to_lowercase().contains(&dd_lower));
            let tint = fs::read_to_string(self.tint_file(&slug))
                .ok()
                .map(|s| s.trim().to_string());
            let md = fs::metadata(&data_dir).ok();
            let created = md.as_ref().and_then(|m| m.created().ok()).and_then(to_secs);
            let last_active = md.as_ref().and_then(|m| m.modified().ok()).and_then(to_secs);
            out.push(Profile {
                name,
                slug,
                app_path: p.display().to_string(),
                data_path: data_dir.display().to_string(),
                running,
                data_size: "—".into(),
                tint,
                created,
                last_active,
            });
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    fn launch(&self, name: &str) -> Result<(), String> {
        let slug = slugify(name);
        let cmd = self.launcher_cmd(&slug);
        if !cmd.exists() {
            return Err(format!("No such profile: {name}"));
        }
        // Prefer the .lnk (carries the AUMID + icon); fall back to the raw .cmd.
        let lnk = self.lnk_path(name);
        let target = if lnk.exists() { lnk } else { cmd };
        let (ok, e) = run("cmd", &["/C", "start", "", &target.display().to_string()]);
        if ok {
            Ok(())
        } else {
            Err(e)
        }
    }

    fn quit(&self, name: &str) -> Result<(), String> {
        let data_dir = self.data_root().join(slugify(name));
        let pids = self.profile_pids(&data_dir);
        if pids.is_empty() {
            return Ok(());
        }
        // Graceful close (tree), wait, then force any survivors.
        for pid in &pids {
            run("taskkill", &["/PID", pid, "/T"]);
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
        for pid in self.profile_pids(&data_dir) {
            run("taskkill", &["/PID", &pid, "/T", "/F"]);
        }
        Ok(())
    }

    fn delete(&self, name: &str, purge: bool) -> Result<(), String> {
        let slug = slugify(name);
        let _ = fs::remove_file(self.launcher_cmd(&slug));
        let _ = fs::remove_file(self.name_file(&slug));
        let _ = fs::remove_file(self.ico_file(&slug));
        let _ = fs::remove_file(self.lnk_path(name));
        if purge {
            let dd = self.data_root().join(&slug);
            if dd.exists() {
                fs::remove_dir_all(&dd).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn repair(&self) -> Result<usize, String> {
        // Re-resolve Claude.exe and rewrite launchers — fixes launchers that point
        // at an app-<ver> path Claude has since updated away from.
        let exe = self.claude_exe().ok_or("Claude Desktop not found.")?;
        let rd = match fs::read_dir(self.app_root()) {
            Ok(r) => r,
            Err(_) => return Ok(0),
        };
        let mut n = 0;
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map_or(false, |x| x == "cmd") {
                let slug = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let data_dir = self.data_root().join(&slug);
                self.write_launcher(&slug, &data_dir, &exe)?;
                n += 1;
            }
        }
        Ok(n)
    }

    fn set_color(&self, name: &str, color: &str) -> Result<(), String> {
        let slug = slugify(name);
        if !self.launcher_cmd(&slug).exists() {
            return Err(format!("No such profile: {name}"));
        }
        let hex = resolve_color(color).ok_or(format!("Unrecognized color '{color}'."))?;
        let _ = fs::write(self.tint_file(&slug), format!("#{hex}"));
        // Best-effort: re-tint the .ico so the shortcut icon updates too.
        if let Some(exe) = self.claude_exe() {
            let (_, tint_ps) = self.ensure_helpers().unwrap_or_default();
            if !tint_ps.as_os_str().is_empty() {
                let ico = self.ico_file(&slug);
                let _ = self.run_ps(
                    &tint_ps,
                    &[
                        ("Exe", &exe.display().to_string()),
                        ("Out", &ico.display().to_string()),
                        ("Hex", &hex),
                    ],
                );
            }
        }
        Ok(())
    }
}
