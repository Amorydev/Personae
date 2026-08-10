// Shared, OS-agnostic types + the Platform trait. Each OS provides an impl.
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const BUNDLE_PREFIX: &str = "com.claudeprofiles";
pub const PALETTE: [&str; 11] = [
    "C2714F", "3B82F6", "22C55E", "8B5CF6", "EC4899",
    "14B8A6", "F59E0B", "EF4444", "6366F1", "06B6D4", "F97316",
];

#[derive(Serialize, Clone)]
pub struct Profile {
    pub name: String,
    pub slug: String,
    pub app_path: String,
    pub data_path: String,
    pub running: bool,
    pub data_size: String,
    pub tint: Option<String>,     // "#RRGGBB"
    pub created: Option<u64>,     // unix secs (data-dir birthtime)
    pub last_active: Option<u64>, // unix secs (data-dir mtime)
}

pub trait Platform {
    fn create(&self, name: &str, color: Option<String>, isolation: &str) -> Result<(), String>;
    fn list(&self) -> Vec<Profile>;
    fn launch(&self, name: &str) -> Result<(), String>;
    fn quit(&self, name: &str) -> Result<(), String>;
    fn delete(&self, name: &str, purge: bool) -> Result<(), String>;
    fn repair(&self) -> Result<usize, String>;
    fn set_color(&self, name: &str, color: &str) -> Result<(), String>;
    fn claude_found(&self) -> bool;
}

// --- shared helpers -------------------------------------------------------

#[allow(dead_code)]
pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

pub fn slugify(s: &str) -> String {
    // Parity with the bash engine: lowercase, map space/underscore to '-', strip
    // anything not [a-z0-9-], COLLAPSE runs of '-' into one, then trim '-' ends.
    // The collapse step is essential — the slug is the data-dir name, so it must
    // match what the classic CLI produced (e.g. "Work - Home" -> "work-home").
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.to_lowercase().chars() {
        let mapped = if c == ' ' || c == '_' { '-' } else { c };
        if mapped == '-' {
            if !prev_dash {
                out.push('-');
                prev_dash = true;
            }
        } else if mapped.is_ascii_lowercase() || mapped.is_ascii_digit() {
            out.push(mapped);
            prev_dash = false;
        }
        // otherwise: stripped char — skip without breaking the dash run,
        // matching bash's strip-then-collapse ordering.
    }
    out.trim_matches('-').to_string()
}

pub fn resolve_color(input: &str) -> Option<String> {
    let c = input.trim().to_lowercase();
    let hex = match c.as_str() {
        "blue" => "3B82F6", "green" => "22C55E", "purple" => "8B5CF6",
        "pink" => "EC4899", "teal" => "14B8A6", "amber" | "yellow" => "F59E0B",
        "red" => "EF4444", "indigo" => "6366F1", "cyan" => "06B6D4", "orange" => "F97316",
        "terracotta" | "clay" => "C2714F",
        other => other.trim_start_matches('#'),
    }
    .to_uppercase();
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex)
    } else {
        None
    }
}

/// SystemTime -> unix seconds (None if before the epoch / unavailable).
pub fn to_secs(t: std::time::SystemTime) -> Option<u64> {
    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
}

/// Run a command, returning (success, combined stdout+stderr).
pub fn run(program: &str, args: &[&str]) -> (bool, String) {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    match cmd.output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.success(), s)
        }
        Err(e) => (false, format!("{e}")),
    }
}

// --- active platform selection -------------------------------------------

#[cfg(target_os = "macos")]
pub fn active() -> crate::macos::Mac {
    crate::macos::Mac::new()
}

#[cfg(windows)]
pub fn active() -> crate::windows::Win {
    crate::windows::Win::new()
}
