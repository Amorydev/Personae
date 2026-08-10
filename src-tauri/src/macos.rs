// macOS implementation — a faithful Rust port of the proven bash engine.
#![cfg(target_os = "macos")]
use crate::platform::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub struct Mac;

impl Mac {
    pub fn new() -> Self { Mac }

    fn app_root(&self) -> PathBuf { home().join("Applications/Claude Profiles") }
    fn data_root(&self) -> PathBuf { home().join("Library/Application Support/ClaudeProfiles") }

    fn claude_app(&self) -> Option<PathBuf> {
        if let Ok(p) = std::env::var("CLAUDE_APP") {
            let pb = PathBuf::from(p);
            if pb.join("Contents/MacOS/Claude").is_file() { return Some(pb); }
        }
        for c in ["/Applications/Claude.app", "~/Applications/Claude.app"] {
            let pb = if let Some(rest) = c.strip_prefix("~/") { home().join(rest) }
                     else { PathBuf::from(c) };
            if pb.join("Contents/MacOS/Claude").is_file() { return Some(pb); }
        }
        None
    }
    fn claude_bin(&self) -> Option<PathBuf> {
        self.claude_app().map(|a| a.join("Contents/MacOS/Claude"))
    }
    fn base_icon(&self) -> Option<PathBuf> {
        self.claude_app().map(|a| a.join("Contents/Resources/electron.icns"))
            .filter(|p| p.is_file())
    }
    // The compiled Swift tinter from the classic tool (bundle later as a sidecar).
    fn icontint(&self) -> Option<PathBuf> {
        let p = home().join("Project/MultiClaudeClone/bin/icontint");
        if p.is_file() { Some(p) } else { None }
    }

    fn tint_file(&self, slug: &str) -> PathBuf {
        self.data_root().join(slug).join(".claude-profile-tint")
    }

    fn count_apps(&self) -> usize {
        fs::read_dir(self.app_root()).map(|rd| {
            rd.flatten().filter(|e| e.path().extension().map_or(false, |x| x == "app")).count()
        }).unwrap_or(0)
    }

    fn write_info_plist(&self, app_dir: &PathBuf, name: &str, bundle: &str) -> Result<(), String> {
        let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>{name}</string>
  <key>CFBundleDisplayName</key><string>{name}</string>
  <key>CFBundleIdentifier</key><string>{bundle}</string>
  <key>CFBundleExecutable</key><string>launcher</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1.0</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>LSMinimumSystemVersion</key><string>10.13</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSArchitecturePriority</key>
  <array><string>arm64</string><string>x86_64</string></array>
</dict>
</plist>
"#);
        fs::write(app_dir.join("Contents/Info.plist"), plist).map_err(|e| e.to_string())
    }

    fn write_launcher(&self, app_dir: &PathBuf, data_dir: &PathBuf, bin: &PathBuf, isolation: &str) -> Result<(), String> {
        let dd = data_dir.display();
        let b = bin.display();
        // Force native arm64 (avoids Rosetta on Apple Silicon; universal binary).
        let body = if isolation == "flag" {
            format!("#!/bin/bash\nBIN=\"{b}\"\nif [ \"$(sysctl -n hw.optional.arm64 2>/dev/null)\" = \"1\" ]; then\n  exec arch -arm64 \"$BIN\" --user-data-dir=\"{dd}\" \"$@\"\nfi\nexec \"$BIN\" --user-data-dir=\"{dd}\" \"$@\"\n")
        } else {
            format!("#!/bin/bash\nexport CLAUDE_USER_DATA_DIR=\"{dd}\"\nBIN=\"{b}\"\nif [ \"$(sysctl -n hw.optional.arm64 2>/dev/null)\" = \"1\" ]; then\n  exec arch -arm64 \"$BIN\" \"$@\"\nfi\nexec \"$BIN\" \"$@\"\n")
        };
        let lp = app_dir.join("Contents/MacOS/launcher");
        fs::write(&lp, body).map_err(|e| e.to_string())?;
        let mut perm = fs::metadata(&lp).map_err(|e| e.to_string())?.permissions();
        perm.set_mode(0o755);
        fs::set_permissions(&lp, perm).map_err(|e| e.to_string())
    }

    fn install_icon(&self, app_dir: &PathBuf, hex: &str) -> Option<()> {
        let dst = app_dir.join("Contents/Resources/icon.icns");
        let base = self.base_icon()?;
        if let Some(tint) = self.icontint() {
            let (ok, _) = run(tint.to_str()?, &[base.to_str()?, dst.to_str()?, &format!("#{hex}")]);
            if ok { return Some(()); }
        }
        fs::copy(&base, &dst).ok().map(|_| ())
    }

    fn lsregister(&self, app_dir: &PathBuf) {
        let ls = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
        if let Some(p) = app_dir.to_str() { run(ls, &["-f", p]); }
    }

    /// All Claude pids (main + helpers) belonging to this profile's data dir.
    /// Robust to Claude living outside /Applications: match the *detected* binary
    /// path (the launcher exec's exactly that), so running-detection works wherever
    /// Claude is installed. `ps eww` includes the environment, so this matches both
    /// env-isolation (CLAUDE_USER_DATA_DIR in env) and flag-isolation (--user-data-dir).
    fn profile_pids(&self, data_dir: &PathBuf) -> Vec<String> {
        let pat = self
            .claude_bin()
            .map(|b| b.display().to_string())
            .unwrap_or_else(|| "Claude.app/Contents/MacOS/Claude".to_string());
        let (_, out) = run("pgrep", &["-f", &pat]);
        let dd = data_dir.display().to_string();
        out.split_whitespace()
            .filter(|pid| {
                let (_, env) = run("ps", &["eww", "-o", "command=", "-p", pid]);
                env.contains(&dd)
            })
            .map(|s| s.to_string())
            .collect()
    }
}

impl Platform for Mac {
    fn claude_found(&self) -> bool { self.claude_bin().is_some() }

    fn create(&self, name: &str, color: Option<String>, isolation: &str) -> Result<(), String> {
        let bin = self.claude_bin().ok_or("Claude Desktop not found (set CLAUDE_APP).")?;
        let slug = slugify(name);
        if slug.is_empty() { return Err("Name must contain letters or numbers.".into()); }
        let iso = if isolation == "flag" { "flag" } else { "env" };

        let app_dir = self.app_root().join(format!("{name}.app"));
        let data_dir = self.data_root().join(&slug);
        let bundle = format!("{BUNDLE_PREFIX}.{slug}");
        if app_dir.exists() { return Err(format!("Profile already exists: {}", app_dir.display())); }

        let pre_count = self.count_apps();
        for d in ["Contents/MacOS", "Contents/Resources"] {
            fs::create_dir_all(app_dir.join(d)).map_err(|e| e.to_string())?;
        }
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

        self.write_info_plist(&app_dir, name, &bundle)?;
        self.write_launcher(&app_dir, &data_dir, &bin, iso)?;

        let hex = match color {
            Some(c) => resolve_color(&c).ok_or(format!("Unrecognized color '{c}'."))?,
            None => PALETTE[pre_count % PALETTE.len()].to_string(),
        };
        self.install_icon(&app_dir, &hex);
        let _ = fs::write(self.tint_file(&slug), format!("#{hex}"));

        self.lsregister(&app_dir);
        Ok(())
    }

    fn list(&self) -> Vec<Profile> {
        let mut out = vec![];
        let rd = match fs::read_dir(self.app_root()) { Ok(r) => r, Err(_) => return out };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map_or(false, |x| x == "app") {
                let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let slug = slugify(&name);
                let data_dir = self.data_root().join(&slug);
                let running = !self.profile_pids(&data_dir).is_empty();
                let data_size = if data_dir.exists() {
                    let (_, o) = run("du", &["-sh", &data_dir.display().to_string()]);
                    o.split('\t').next().unwrap_or("—").trim().to_string()
                } else { "—".into() };
                let tint = fs::read_to_string(self.tint_file(&slug)).ok()
                    .map(|s| s.trim().to_string());
                let md = fs::metadata(&data_dir).ok();
                let created = md.as_ref().and_then(|m| m.created().ok()).and_then(to_secs);
                let last_active = md.as_ref().and_then(|m| m.modified().ok()).and_then(to_secs);
                out.push(Profile {
                    name, slug,
                    app_path: p.display().to_string(),
                    data_path: data_dir.display().to_string(),
                    running, data_size, tint, created, last_active,
                });
            }
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    fn launch(&self, name: &str) -> Result<(), String> {
        let app = self.app_root().join(format!("{name}.app"));
        if !app.exists() { return Err(format!("No such profile: {name}")); }
        let (ok, e) = run("open", &[&app.display().to_string()]);
        if ok { Ok(()) } else { Err(e) }
    }

    fn quit(&self, name: &str) -> Result<(), String> {
        let slug = slugify(name);
        let bundle = format!("{BUNDLE_PREFIX}.{slug}");
        let data_dir = self.data_root().join(&slug);
        if self.profile_pids(&data_dir).is_empty() { return Ok(()); }
        // graceful
        let _ = run("osascript", &["-e", &format!("tell application id \"{bundle}\" to quit")]);
        for _ in 0..6 {
            if self.profile_pids(&data_dir).is_empty() { return Ok(()); }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        // TERM then KILL (targets main + helpers by data dir)
        let pids = self.profile_pids(&data_dir);
        if !pids.is_empty() {
            let mut a = vec![]; for p in &pids { a.push(p.as_str()); }
            run("kill", &a);
        }
        for _ in 0..4 {
            if self.profile_pids(&data_dir).is_empty() { return Ok(()); }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        let pids = self.profile_pids(&data_dir);
        if !pids.is_empty() {
            let mut a = vec!["-9"]; for p in &pids { a.push(p.as_str()); }
            run("kill", &a);
        }
        Ok(())
    }

    fn delete(&self, name: &str, purge: bool) -> Result<(), String> {
        let app = self.app_root().join(format!("{name}.app"));
        let data_dir = self.data_root().join(slugify(name));
        if !app.exists() { return Err(format!("No such profile: {name}")); }
        fs::remove_dir_all(&app).map_err(|e| e.to_string())?;
        if purge && data_dir.exists() {
            fs::remove_dir_all(&data_dir).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn repair(&self) -> Result<usize, String> {
        let bin = self.claude_bin().ok_or("Claude Desktop not found.")?;
        let rd = match fs::read_dir(self.app_root()) { Ok(r) => r, Err(_) => return Ok(0) };
        let mut n = 0;
        for e in rd.flatten() {
            let app = e.path();
            if app.extension().map_or(false, |x| x == "app") {
                let name = app.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let slug = slugify(&name);
                let data_dir = self.data_root().join(&slug);
                let bundle = format!("{BUNDLE_PREFIX}.{slug}");
                let launcher = app.join("Contents/MacOS/launcher");
                let iso = fs::read_to_string(&launcher).map(|s| if s.contains("--user-data-dir=") { "flag" } else { "env" }).unwrap_or("env");
                self.write_launcher(&app, &data_dir, &bin, iso)?;
                self.write_info_plist(&app, &name, &bundle)?;
                self.lsregister(&app);
                n += 1;
            }
        }
        Ok(n)
    }

    fn set_color(&self, name: &str, color: &str) -> Result<(), String> {
        let slug = slugify(name);
        let app_dir = self.app_root().join(format!("{name}.app"));
        if !app_dir.exists() { return Err(format!("No such profile: {name}")); }
        let hex = resolve_color(color).ok_or(format!("Unrecognized color '{color}'."))?;
        self.install_icon(&app_dir, &hex);
        let _ = fs::write(self.tint_file(&slug), format!("#{hex}"));
        self.lsregister(&app_dir);
        Ok(())
    }
}
