// Claude Code CLI profile engine. Parallels macos.rs (Desktop), but isolates the
// *CLI*: each profile is a real Claude login, seeded by a one-time `/login`
// inside its own CLAUDE_CONFIG_DIR. Claude Code itself namespaces the OAuth
// credential in the macOS Keychain by a hash of CLAUDE_CONFIG_DIR (service
// `Claude Code-credentials-<sha256(config_dir)[:8]>`), so distinct config dirs
// give distinct, durable, concurrently-usable logins — with zero app-managed
// secrets. No token injection, no shim, no setup-token capture.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct CliProfile {
    pub name: String,
    pub slug: String,
    pub config_dir: String,    // CLAUDE_CONFIG_DIR for this profile
    pub launcher_path: String, // the generated .command
    pub logged_in: bool,       // an oauthAccount exists in <config_dir>/.claude.json, OR an api_key provider is configured
    pub account_email: Option<String>, // parsed from <config_dir>/.claude.json if present
    pub data_size: String,
    pub created: Option<u64>,
    pub last_active: Option<u64>,
    pub auth_mode: String,             // "oauth" | "api_key" — from <config_dir>/settings.json
    pub provider_model: Option<String>, // display-only: the configured model, if any
    // OAuth token lifetimes, unix ms, parsed from <config_dir>/.credentials.json's
    // claudeAiOauth object (Windows only — macOS stores this in the Keychain, not
    // a plaintext file, so these stay None there rather than dumping a secret
    // just to read a non-secret expiry). `token_expires_at` is the short-lived
    // access token (silently auto-refreshed by claude; not user-actionable).
    // `refresh_expires_at` is what actually requires a real re-login once it lapses.
    pub token_expires_at: Option<u64>,
    pub refresh_expires_at: Option<u64>,
    // Plan/quota tier — also from claudeAiOauth (same Windows-only caveat as
    // the expiry fields above). This is the account's *plan tier*, a label
    // like "max"/"default_claude_max_20x", not a live number.
    pub subscription_type: Option<String>, // e.g. "max", "pro"
    pub rate_limit_tier: Option<String>,   // e.g. "default_claude_max_20x"
    // Real, live usage percentages — parsed from `<config_dir>/.claude.json`'s
    // `cachedUsageUtilization.utilization` object (present on both platforms;
    // the CLI itself populates and periodically refreshes this cache, not
    // Personae). `None` until the account has made at least one real request
    // (never fabricated). `session_usage_pct` is the rolling 5-hour window;
    // `weekly_usage_pct` is `utilization.limits[].percent` for the
    // `kind: "weekly_all"` entry (the plan's overall weekly cap, distinct from
    // a per-model `weekly_scoped` entry).
    pub session_usage_pct: Option<u32>,
    pub weekly_usage_pct: Option<u32>,
}

/// Parse `claudeAiOauth.expiresAt` / `.refreshTokenExpiresAt` (unix ms) out of a
/// `.credentials.json` body. Returns `(None, None)` for malformed/absent input —
/// callers treat that the same as "unknown", not an error.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
fn extract_token_expiry(json: &str) -> (Option<u64>, Option<u64>) {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let oauth = v.get("claudeAiOauth");
    let expires_at = oauth.and_then(|o| o.get("expiresAt")).and_then(|x| x.as_u64());
    let refresh_expires_at = oauth.and_then(|o| o.get("refreshTokenExpiresAt")).and_then(|x| x.as_u64());
    (expires_at, refresh_expires_at)
}

/// Parse `claudeAiOauth.subscriptionType` / `.rateLimitTier` out of a
/// `.credentials.json` body — real fields confirmed present on a live account
/// this session. `(None, None)` for malformed/absent input (e.g. api-key-mode
/// accounts have no claudeAiOauth object at all).
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
fn extract_plan_tier(json: &str) -> (Option<String>, Option<String>) {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let oauth = v.get("claudeAiOauth");
    let subscription_type = oauth.and_then(|o| o.get("subscriptionType")).and_then(|x| x.as_str()).map(String::from);
    let rate_limit_tier = oauth.and_then(|o| o.get("rateLimitTier")).and_then(|x| x.as_str()).map(String::from);
    (subscription_type, rate_limit_tier)
}

/// The curated subset of `<config_dir>/settings.json` this app edits: enough to
/// point a profile at a custom Anthropic-API-compatible provider (proxy, DeepSeek
/// gateway, etc.) via env vars Claude Code itself already reads. Any other keys a
/// user hand-edited into settings.json (permissions, theme, plugins, ...) are
/// preserved untouched — see `save_provider_config`.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProviderConfig {
    pub auth_mode: String, // "oauth" | "api_key"
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub small_fast_model: Option<String>,
}

fn provider_settings_file(config_dir: &Path) -> PathBuf { config_dir.join("settings.json") }

fn load_provider_config(config_dir: &Path) -> ProviderConfig {
    let v: serde_json::Value = std::fs::read_to_string(provider_settings_file(config_dir)).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let env = v.get("env").cloned().unwrap_or_else(|| serde_json::json!({}));
    let get = |k: &str| env.get(k).and_then(|x| x.as_str()).map(|s| s.to_string()).filter(|s| !s.is_empty());
    let api_key = get("ANTHROPIC_AUTH_TOKEN").or_else(|| get("ANTHROPIC_API_KEY"));
    ProviderConfig {
        auth_mode: if api_key.is_some() { "api_key".into() } else { "oauth".into() },
        base_url: get("ANTHROPIC_BASE_URL"),
        api_key,
        model: get("ANTHROPIC_MODEL"),
        small_fast_model: get("ANTHROPIC_SMALL_FAST_MODEL"),
    }
}

/// Read-modify-write `<config_dir>/settings.json`, touching only the `env` keys
/// this app owns. Mirrors `ide::merge_vscode_settings`'s preserve-everything-else
/// approach. Switching back to `oauth` clears the api-key-mode env overrides so a
/// stale token/base-url doesn't linger and silently redirect `claude auth login`.
fn save_provider_config(config_dir: &Path, cfg: &ProviderConfig) -> Result<(), String> {
    let f = provider_settings_file(config_dir);
    let mut root: serde_json::Value = match std::fs::read_to_string(&f) {
        Ok(s) if !s.trim().is_empty() => serde_json::from_str(&s)
            .map_err(|e| format!("existing settings.json is not valid JSON: {e}"))?,
        _ => serde_json::json!({}),
    };
    if !root.is_object() { return Err("settings.json root is not an object".into()); }
    let obj = root.as_object_mut().unwrap();
    let env_entry = obj.entry("env".to_string()).or_insert_with(|| serde_json::json!({}));
    if !env_entry.is_object() { *env_entry = serde_json::json!({}); }
    let env = env_entry.as_object_mut().unwrap();

    let set_or_clear = |env: &mut serde_json::Map<String, serde_json::Value>, key: &str, val: &Option<String>| {
        match val.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(v) => { env.insert(key.into(), serde_json::Value::String(v.to_string())); }
            None => { env.remove(key); }
        }
    };

    if cfg.auth_mode == "api_key" {
        set_or_clear(env, "ANTHROPIC_AUTH_TOKEN", &cfg.api_key);
        set_or_clear(env, "ANTHROPIC_BASE_URL", &cfg.base_url);
        set_or_clear(env, "ANTHROPIC_MODEL", &cfg.model);
        set_or_clear(env, "ANTHROPIC_SMALL_FAST_MODEL", &cfg.small_fast_model);
    } else {
        for key in ["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL", "ANTHROPIC_MODEL", "ANTHROPIC_SMALL_FAST_MODEL"] {
            env.remove(key);
        }
    }

    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let body = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    std::fs::write(&f, body).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn resolve_config_dir(slug: &str) -> PathBuf { imp::config_dir(slug) }
#[cfg(windows)]
fn resolve_config_dir(slug: &str) -> PathBuf { imp_win::config_dir(slug) }
#[cfg(not(any(target_os = "macos", windows)))]
fn resolve_config_dir(slug: &str) -> PathBuf { config_root().join(slug) }

#[cfg(target_os = "macos")]
fn read_credentials_json(slug: &str) -> Option<String> { imp::read_login_credentials(slug) }
#[cfg(not(target_os = "macos"))]
fn read_credentials_json(slug: &str) -> Option<String> {
    std::fs::read_to_string(resolve_config_dir(slug).join(".credentials.json")).ok()
}

/// Resolves a display *name* to its real slug. NOT just `slugify(name)`: a
/// renamed account's slug can diverge from its current display name — on
/// Windows it always does once renamed (`rename` deliberately keeps the slug
/// fixed to the account's original creation name, so config_dir/credentials
/// never have to move); on macOS `rename` keeps them in sync by design (moves
/// config_dir + the Keychain entry when the slug would change), so the naive
/// form is always correct there. Every lookup-by-name function should go
/// through this rather than calling `slugify(name)` directly — that bug
/// meant `delete`/`login`/`launch`/etc. on Windows could never find a
/// renamed account again, confirmed live: deleting a renamed throwaway
/// account failed with "No such CLI account" even though it still existed on
/// disk under its original slug.
#[cfg(target_os = "macos")]
pub(crate) fn resolve_slug(name: &str) -> String { crate::platform::slugify(name) }
#[cfg(windows)]
pub(crate) fn resolve_slug(name: &str) -> String { imp_win::resolve_slug(name) }
#[cfg(not(any(target_os = "macos", windows)))]
pub(crate) fn resolve_slug(name: &str) -> String { crate::platform::slugify(name) }

pub fn get_provider_config(name: &str) -> Result<ProviderConfig, String> {
    let slug = resolve_slug(name);
    let dir = resolve_config_dir(&slug);
    if !dir.exists() { return Err(format!("No such CLI account: {name}")); }
    Ok(load_provider_config(&dir))
}

pub fn set_provider_config(name: &str, cfg: ProviderConfig) -> Result<(), String> {
    let slug = resolve_slug(name);
    let dir = resolve_config_dir(&slug);
    if !dir.exists() { return Err(format!("No such CLI account: {name}")); }
    save_provider_config(&dir, &cfg)
}

/// Escape a string for safe interpolation inside a double-quoted bash string.
pub fn sh_dq_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Wrap a string as a safe POSIX single-quoted shell word (handles embedded ').
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The bash body of a profile launcher. Sets the profile's CLAUDE_CONFIG_DIR and
/// execs the real claude CLI — claude resolves the per-profile login from its
/// own Keychain slot (namespaced by config dir). On first run claude shows the
/// login prompt; that one-time /login seeds the profile. Also clears
/// CLAUDE_CODE_CHILD_SESSION: if Personae itself happens to be running under a
/// Claude Code session (e.g. launched from its Bash tool during development),
/// that marker is otherwise inherited by every process Personae spawns,
/// including this launcher — the nested `claude` then silently disables its
/// own transcript saving. Isolation is the whole point of this app, so a
/// spawned account session should never inherit stray state from whatever
/// shell happened to start Personae.
pub fn render_launcher(name: &str, config_dir: &str, claude_bin: &str) -> String {
    let name_e = sh_dq_escape(name);
    let cfg_e = sh_dq_escape(config_dir);
    let bin_e = sh_dq_escape(claude_bin);
    format!(
        "#!/bin/bash\n\
         # Generated by Claude Profiles — CLI account launcher for \"{name_e}\".\n\
         export CLAUDE_CONFIG_DIR=\"{cfg_e}\"\n\
         CLAUDE_BIN=\"{bin_e}\"\n\
         [ -x \"$CLAUDE_BIN\" ] || CLAUDE_BIN=\"$(command -v claude)\"\n\
         unset CLAUDE_CODE_CHILD_SESSION\n\
         printf \"\\033]0;Personae — {name_e}\\007\"\n\
         echo \"Personae account: {name_e}\"\n\
         exec \"$CLAUDE_BIN\" \"$@\"\n"
    )
}

/// The Keychain service name claude uses for a login stored under `config_dir`
/// (namespaced by config-dir hash). Pure + testable — verified against a real
/// profile this session (see test).
pub fn login_keychain_service_for_dir(config_dir: &str) -> String {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(config_dir.as_bytes());
    let hex: String = h.iter().take(4).map(|b| format!("{b:02x}")).collect(); // first 8 hex chars
    format!("Claude Code-credentials-{hex}")
}

/// Cheap extract of oauthAccount.emailAddress from a .claude.json body without
/// pulling in serde_json.
pub fn extract_email(json: &str) -> Option<String> {
    let key = "\"emailAddress\"";
    let i = json.find(key)? + key.len();
    let rest = &json[i..];
    let start = rest.find('"')? + 1;
    let after = &rest[start..];
    let end = after.find('"')?;
    let email = &after[..end];
    if email.contains('@') { Some(email.to_string()) } else { None }
}

/// Parse live usage percentages out of a `.claude.json` body's
/// `cachedUsageUtilization.utilization` object — shape confirmed live against
/// a real account this session: `.five_hour.utilization` (0-100, rolling
/// session window) and `.limits[]` (an array of `{kind, group, percent, ...}`
/// entries; `kind: "weekly_all"` is the plan's overall weekly cap). Returns
/// `(None, None)` for malformed/absent input, e.g. an account that has never
/// made a real request yet, or an API-key-mode account (no Claude usage cap).
pub fn extract_usage_utilization(json: &str) -> (Option<u32>, Option<u32>) {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    parse_utilization_percentages(v.get("cachedUsageUtilization").and_then(|c| c.get("utilization")))
}

/// Shared by `extract_usage_utilization` (the CLI's cached copy, nested under
/// `cachedUsageUtilization.utilization` in `.claude.json`) and
/// `fetch_live_usage` (the raw response body IS this same "utilization"
/// shape). `as_f64` + round, not `as_u64`: the CLI's own cache serializes a
/// whole-number percent bare (`29`), but the live API returns it as an
/// explicit float (`29.0`) — `as_u64()` would silently miss the latter.
fn parse_utilization_percentages(util: Option<&serde_json::Value>) -> (Option<u32>, Option<u32>) {
    let session = util
        .and_then(|u| u.get("five_hour"))
        .and_then(|f| f.get("utilization"))
        .and_then(|x| x.as_f64())
        .map(|x| x.round() as u32);
    let weekly = util
        .and_then(|u| u.get("limits"))
        .and_then(|l| l.as_array())
        .and_then(|arr| arr.iter().find(|e| e.get("kind").and_then(|k| k.as_str()) == Some("weekly_all")))
        .and_then(|e| e.get("percent"))
        .and_then(|x| x.as_f64())
        .map(|x| x.round() as u32);
    (session, weekly)
}

/// Fetch **live** usage straight from Anthropic's own endpoint — the exact
/// call `claude` itself makes (confirmed by tracing the bundled CLI's own
/// source this session, then verifying live with a real GET): a lightweight,
/// read-only status fetch (5s timeout in the CLI's own code), not a billed
/// completion. Used only for the explicit manual refresh button — the
/// automatic refreshes (account select, window focus, switching into the CLI
/// view) stay on the cheap local re-read (`get_usage`) so normal browsing
/// never silently calls out to this endpoint on every click.
///
/// The access token is written to a short-lived curl `-K` config file rather
/// than passed as a `-H` argument: a process's command-line arguments are
/// readable by other processes on the machine (e.g. via WMI), which would
/// expose a live bearer token more broadly than this platform's existing,
/// already-accepted exposure (the plaintext `.credentials.json` itself — see
/// `imp_win`'s module doc for why: no Keychain on Windows).
pub fn fetch_live_usage(name: &str) -> Result<(Option<u32>, Option<u32>), String> {
    let slug = resolve_slug(name);
    let creds = read_credentials_json(&slug)
        .ok_or_else(|| "No stored credentials for this account.".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&creds).map_err(|e| e.to_string())?;
    let token = v.get("claudeAiOauth").and_then(|o| o.get("accessToken")).and_then(|t| t.as_str())
        .ok_or("This account has no OAuth access token (API-key accounts have no Claude usage cap).")?;

    // `write-out` appends the HTTP status after the body, marked so it can be
    // split back off reliably — curl's own exit code only reflects transport
    // success, not the HTTP status, so without this a 401 (expired token)
    // would silently fall through to the "unexpected response" branch below
    // instead of being distinguished as an auth failure.
    let cfg_path = std::env::temp_dir().join(format!("personae-usage-{}-{}.curlcfg", std::process::id(), slug));
    let cfg_body = format!(
        "silent\nshow-error\nheader = \"Content-Type: application/json\"\nheader = \"Authorization: Bearer {token}\"\nurl = \"https://api.anthropic.com/api/oauth/usage\"\nwrite-out = \"HTTP_STATUS:%{{http_code}}\"\n"
    );
    std::fs::write(&cfg_path, cfg_body).map_err(|e| e.to_string())?;
    let (ok, out) = crate::platform::run("curl", &["-K", &cfg_path.display().to_string(), "--max-time", "8"]);
    let _ = std::fs::remove_file(&cfg_path);

    if !ok { return Err(format!("Usage fetch failed: {}", out.trim())); }

    let (body_str, status) = match out.rsplit_once("HTTP_STATUS:") {
        Some((b, s)) => (b, s.trim().parse::<u16>().unwrap_or(0)),
        None => (out.as_str(), 0),
    };
    // Sentinel string, not a formatted message: the frontend pattern-matches
    // this exact value to trigger its "session expired, please sign in
    // again" UI rather than showing it as a raw error toast.
    if status == 401 { return Err("AUTH_EXPIRED".to_string()); }
    if status != 0 && status >= 400 {
        return Err(format!("Usage fetch failed (HTTP {status}): {}", body_str.trim().chars().take(200).collect::<String>()));
    }
    let body: serde_json::Value = serde_json::from_str(body_str)
        .map_err(|_| format!("Unexpected response: {}", body_str.trim().chars().take(200).collect::<String>()))?;
    Ok(parse_utilization_percentages(Some(&body)))
}

// ---- cross-platform Windows-launcher builders (pure; unit-tested on macOS) --

/// Double `%` so it is literal inside a batch `set "VAR=…"`. NOT a
/// general-purpose escaper: it is sufficient only for the constrained values we
/// pass (config-dir paths under `%APPDATA%` with `[a-z0-9-]` slugs, and the
/// resolved claude bin path). Within `set "…"` the other cmd metachars
/// (`& ^ < > |`) are already literal, so `%` is the only hazard for those inputs.
#[allow(dead_code)] // used on Windows + in tests; unreferenced in the macOS build
pub fn cmd_set_value_escape(s: &str) -> String { s.replace('%', "%%") }

/// Path, relative to the npm global bin dir (where the `claude` shim lives), of
/// the native `claude.exe` that `@anthropic-ai/claude-code` unpacks for
/// win32-x64. Last-resort resolver fallback: a Windows field report found the
/// npm `.ps1` shim can carry a broken path, and a Tauri GUI may not inherit the
/// shell PATH (so `where claude` misses). Pure + unit-tested on macOS.
#[allow(dead_code)] // used on Windows + in tests; unreferenced in the macOS build
pub fn nested_win_claude_exe(npm_bin_dir: &Path) -> PathBuf {
    npm_bin_dir
        .join("node_modules").join("@anthropic-ai").join("claude-code")
        .join("node_modules").join("@anthropic-ai").join("claude-code-win32-x64")
        .join("claude.exe")
}

/// Sanitize a display name for a batch `REM` comment: strip CR/LF (which would
/// end the comment line) and double `%`.
#[allow(dead_code)] // used on Windows + in tests; unreferenced in the macOS build
fn cmd_comment_sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '\r' && *c != '\n')
        .collect::<String>()
        .replace('%', "%%")
}

/// Windows `.cmd` launcher: set this account's CLAUDE_CONFIG_DIR, then `call`
/// the real claude (`call`, because claude is usually a `.cmd` shim; without
/// `call` control would not return to this script). CRLF line endings for cmd.
/// Also clears CLAUDE_CODE_CHILD_SESSION — see `render_launcher`'s doc comment
/// for why (isolation: a spawned account session must not inherit stray state
/// from whatever shell happened to start Personae itself).
#[allow(dead_code)] // used on Windows + in tests; unreferenced in the macOS build
pub fn render_launcher_win(name: &str, config_dir: &str, claude_bin: &str) -> String {
    let name_c = cmd_comment_sanitize(name);
    let cfg = cmd_set_value_escape(config_dir);
    let bin = cmd_set_value_escape(claude_bin);
    // Parity with the macOS launcher: fall back to PATH `claude` if the baked
    // path is stale (Claude moved/updated), so the launcher self-heals.
    format!(
        "@echo off\r\n\
         REM Generated by Personae — CLI account launcher for \"{name_c}\".\r\n\
         title Personae - {name_c}\r\n\
         set \"CLAUDE_CONFIG_DIR={cfg}\"\r\n\
         set \"CLAUDE_BIN={bin}\"\r\n\
         if not exist \"%CLAUDE_BIN%\" set \"CLAUDE_BIN=claude\"\r\n\
         set \"CLAUDE_CODE_CHILD_SESSION=\"\r\n\
         echo Personae account: {name_c}\r\n\
         call \"%CLAUDE_BIN%\" %*\r\n"
    )
}

/// One-shot interactive sign-in window: set env, run `claude auth login`,
/// keep the console open so the user sees the result.
#[allow(dead_code)] // used on Windows + in tests; unreferenced in the macOS build
/// `browser`, if set, is written as `set "BROWSER=<path>"` so `claude auth
/// login` opens that program for the OAuth link instead of the system
/// default — see `browser.rs`.
pub fn render_login_win(name: &str, config_dir: &str, claude_bin: &str, browser: Option<&str>) -> String {
    let name_c = cmd_comment_sanitize(name);
    let cfg = cmd_set_value_escape(config_dir);
    let bin = cmd_set_value_escape(claude_bin);
    let browser_line = browser
        .map(|b| format!("set \"BROWSER={}\"\r\n", cmd_set_value_escape(b)))
        .unwrap_or_default();
    format!(
        "@echo off\r\n\
         set \"CLAUDE_CONFIG_DIR={cfg}\"\r\n\
         set \"CLAUDE_BIN={bin}\"\r\n\
         if not exist \"%CLAUDE_BIN%\" set \"CLAUDE_BIN=claude\"\r\n\
         set \"CLAUDE_CODE_CHILD_SESSION=\"\r\n\
         {browser_line}\
         echo Signing in to Claude Code for \"{name_c}\"...\r\n\
         echo.\r\n\
         call \"%CLAUDE_BIN%\" auth login\r\n\
         echo.\r\n\
         echo Finished. You can close this window and return to Personae.\r\n\
         pause\r\n"
    )
}

// ---- cross-platform onboarding / credential-presence / size helpers --------

/// A stored login exists for this config dir (Windows/Linux signal): claude's
/// plaintext credential file is present. (macOS uses the Keychain slot instead.)
#[allow(dead_code)] // used on Windows + in tests; unreferenced in the macOS build
pub fn credentials_present_at(config_dir: &Path) -> bool {
    config_dir.join(".credentials.json").is_file()
}

/// Set hasCompletedOnboarding:true in <config_dir>/.claude.json (idempotent,
/// best-effort) so interactive `claude` skips the first-run menu. Shared by all
/// OSes; `claude auth login` stores the credential but does NOT set this flag.
pub fn mark_onboarded_at(config_dir: &Path) {
    let f = config_dir.join(".claude.json");
    let mut v: serde_json::Value = std::fs::read_to_string(&f).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if v.get("hasCompletedOnboarding").and_then(|b| b.as_bool()) == Some(true) { return; }
    if let Some(o) = v.as_object_mut() {
        o.insert("hasCompletedOnboarding".into(), serde_json::Value::Bool(true));
    }
    let _ = std::fs::create_dir_all(config_dir);
    if let Ok(s) = serde_json::to_string_pretty(&v) { let _ = std::fs::write(&f, s); }
}

/// Rough recursive byte count (Windows has no `du`); best-effort, ignores errors.
#[allow(dead_code)] // used on Windows (list); unreferenced in the macOS build
fn dir_size_bytes(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let path = e.path();
            match e.metadata() {
                Ok(m) if m.is_dir() => total += dir_size_bytes(&path),
                Ok(m) => total += m.len(),
                _ => {}
            }
        }
    }
    total
}

#[allow(dead_code)] // used on Windows (list); unreferenced in the macOS build
fn human_size(bytes: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let (mut b, mut i) = (bytes as f64, 0);
    while b >= 1024.0 && i < U.len() - 1 { b /= 1024.0; i += 1; }
    if i == 0 { format!("{} {}", bytes, U[0]) } else { format!("{:.1} {}", b, U[i]) }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use crate::platform::{home, run};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    // ~/Applications/Personae/CLI/<name>.command
    pub fn launcher_root() -> PathBuf { home().join("Applications/Personae/CLI") }
    // ~/Library/Application Support/Personae/CLI/<slug>   (== CLAUDE_CONFIG_DIR)
    pub fn config_root() -> PathBuf { home().join("Library/Application Support/Personae/CLI") }

    pub fn launcher_path(name: &str) -> PathBuf { launcher_root().join(format!("{name}.command")) }
    pub fn config_dir(slug: &str) -> PathBuf { config_root().join(slug) }

    /// One-time move from the legacy `ClaudeProfilesCLI`/`Claude Profiles CLI`
    /// folder names to `Personae` — existing accounts' data and launchers are
    /// relocated wholesale via `fs::rename` (same volume, so this is an atomic
    /// move, not a copy: nothing is lost). Idempotent no-op once the new
    /// folders exist; called once per `list()`, mirroring this module's
    /// existing self-heal-on-list pattern.
    ///
    /// CAVEAT (unverified — no Mac to test against): claude's Keychain slot for
    /// a login is namespaced by a hash of the *config-dir path*
    /// (`login_keychain_service_for_dir`). Because this move changes that path,
    /// any already-signed-in macOS account's Keychain entry will no longer be
    /// found under the new path's hash after migration — the account data
    /// itself moves intact, but a re-login would be needed. Windows has no such
    /// hash indirection (the credential is a plaintext file inside config_dir,
    /// see `imp_win`'s module doc), so this caveat is macOS-only.
    pub fn migrate_legacy_dirs() {
        let old_config = home().join("Library/Application Support/ClaudeProfilesCLI");
        let new_config = config_root();
        if old_config.is_dir() && !new_config.exists() {
            if let Some(parent) = new_config.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::rename(&old_config, &new_config);
        }
        let old_launcher = home().join("Applications/Claude Profiles CLI");
        let new_launcher = launcher_root();
        if old_launcher.is_dir() && !new_launcher.exists() {
            if let Some(parent) = new_launcher.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::rename(&old_launcher, &new_launcher);
        }
    }

    /// Resolve the claude CLI binary (env override → PATH → common install dirs).
    pub fn claude_bin() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("CLAUDE_CLI_BIN") {
            let pb = PathBuf::from(p);
            if pb.is_file() { return Some(pb); }
        }
        let (ok, out) = run("which", &["claude"]);
        if ok {
            let p = PathBuf::from(out.trim());
            if p.is_file() { return Some(p); }
        }
        for c in [
            "/opt/homebrew/bin/claude", "/usr/local/bin/claude",
            "~/.local/bin/claude", "~/.claude/local/claude", "~/.bun/bin/claude",
        ] {
            let pb = if let Some(rest) = c.strip_prefix("~/") { home().join(rest) } else { PathBuf::from(c) };
            if pb.is_file() { return Some(pb); }
        }
        None
    }

    pub fn write_launcher(name: &str, slug: &str) -> Result<(), String> {
        let cfg = config_dir(slug);
        let bin = claude_bin().map(|p| p.display().to_string()).unwrap_or_default();
        let body = render_launcher(name, &cfg.display().to_string(), &bin);
        let lp = launcher_path(name);
        fs::create_dir_all(launcher_root()).map_err(|e| e.to_string())?;
        fs::write(&lp, body).map_err(|e| e.to_string())?;
        let mut perm = fs::metadata(&lp).map_err(|e| e.to_string())?.permissions();
        perm.set_mode(0o755);
        fs::set_permissions(&lp, perm).map_err(|e| e.to_string())
    }

    /// The Keychain service claude uses for this profile's login (namespaced by
    /// config-dir hash — mirrors the CLI's own derivation). Used only to purge on
    /// delete. Default config dir would be un-suffixed, but profiles are always
    /// custom, so the hash suffix always applies.
    pub fn login_keychain_service(slug: &str) -> String {
        super::login_keychain_service_for_dir(&config_dir(slug).display().to_string())
    }

    /// Whether a login credential is stored for this profile — checked by the
    /// existence of claude's per-config-dir Keychain slot. This is the reliable
    /// signal: `claude auth status` reports a false negative once the access
    /// token expires (even though the refresh token is still valid and usage
    /// works), and `oauthAccount` in .claude.json is only populated lazily.
    pub fn login_present(slug: &str) -> bool {
        let svc = login_keychain_service(slug);
        let user = std::env::var("USER").unwrap_or_default();
        run("security", &["find-generic-password", "-s", &svc, "-a", &user]).0
    }

    pub fn read_login_credentials(slug: &str) -> Option<String> {
        let svc = login_keychain_service(slug);
        let user = std::env::var("USER").unwrap_or_default();
        let (ok, out) = run("security", &["find-generic-password", "-s", &svc, "-a", &user, "-w"]);
        if ok && !out.trim().is_empty() { Some(out) } else { None }
    }

    /// Mark this profile's config dir as onboarded so interactive `claude` skips
    /// the first-run login/onboarding menu (`claude auth login` stores the
    /// credential but does NOT set this flag, so without it a logged-in profile
    /// still shows the menu). Idempotent + best-effort: only writes when the flag
    /// isn't already true, so it doesn't churn .claude.json on every list.
    pub fn mark_onboarded(slug: &str) {
        super::mark_onboarded_at(&config_dir(slug));
    }
}

// Windows CLI engine — WRITTEN FROM CLAUDE'S CODE + WINDOWS DOCS, NOT YET RUN
// ON REAL WINDOWS. Every process/quoting assumption is marked TODO(verify); see
// docs/WINDOWS-TEST-PLAN.md. Isolation needs ONLY CLAUDE_CONFIG_DIR: on Windows
// there is no Keychain, so claude's credential store falls back to a plaintext
// <CLAUDE_CONFIG_DIR>\.credentials.json — one config dir per account gives one
// durable, concurrently-usable login. No keychain hashing, no token injection.
#[cfg(windows)]
mod imp_win {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn appdata() -> PathBuf { PathBuf::from(std::env::var("APPDATA").unwrap_or_default()) }
    fn localappdata() -> PathBuf { PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()) }
    fn userprofile() -> PathBuf { PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()) }

    // %LOCALAPPDATA%\Personae\CLI\apps\<slug>.cmd  (+ .name sidecar, + _login\<slug>.cmd)
    pub fn launcher_root() -> PathBuf { localappdata().join("Personae").join("CLI").join("apps") }
    // %APPDATA%\Personae\CLI\<slug>   == CLAUDE_CONFIG_DIR
    pub fn config_root() -> PathBuf { appdata().join("Personae").join("CLI") }

    pub fn launcher_path(slug: &str) -> PathBuf { launcher_root().join(format!("{slug}.cmd")) }
    pub fn name_path(slug: &str) -> PathBuf { launcher_root().join(format!("{slug}.name")) }
    pub fn login_path(slug: &str) -> PathBuf { launcher_root().join("_login").join(format!("{slug}.cmd")) }
    pub fn config_dir(slug: &str) -> PathBuf { config_root().join(slug) }

    /// See `super::resolve_slug`'s doc comment for why this exists at all.
    /// Fast path (an unrenamed account, or before any rename has ever
    /// happened) is exactly `slugify(name)`; only falls back to scanning
    /// `.name` sidecars — the only place a renamed account's real slug is
    /// recorded — when that doesn't resolve to a real launcher.
    pub fn resolve_slug(name: &str) -> String {
        let fast = crate::platform::slugify(name);
        if launcher_path(&fast).exists() { return fast; }
        if let Ok(rd) = fs::read_dir(launcher_root()) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map_or(false, |x| x == "name") {
                    if let Ok(stored) = fs::read_to_string(&p) {
                        if stored.trim() == name {
                            if let Some(stem) = p.file_stem() {
                                return stem.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        }
        fast
    }

    /// One-time move from the legacy `ClaudeProfilesCLI` folder name to
    /// `Personae` — see the macOS `imp::migrate_legacy_dirs` doc comment for
    /// the full rationale (same idempotent, self-heal-on-list pattern). No
    /// Keychain-hash caveat on Windows: the credential is a plaintext file
    /// (`.credentials.json`) inside the moved config_dir itself.
    pub fn migrate_legacy_dirs() {
        let old_config = appdata().join("ClaudeProfilesCLI");
        let new_config = config_root();
        if old_config.is_dir() && !new_config.exists() {
            if let Some(parent) = new_config.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::rename(&old_config, &new_config);
        }
        let old_launcher_root = localappdata().join("ClaudeProfilesCLI");
        let new_launcher_root = localappdata().join("Personae").join("CLI");
        if old_launcher_root.is_dir() && !new_launcher_root.exists() {
            if let Some(parent) = new_launcher_root.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::rename(&old_launcher_root, &new_launcher_root);
        }
    }

    /// Resolve the claude CLI: env override → `where claude` → common installs.
    pub fn claude_bin() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("CLAUDE_CLI_BIN") {
            let pb = PathBuf::from(p);
            if pb.is_file() { return Some(pb); }
        }
        // TODO(verify): `where claude` resolves the shim on PATH (npm/bun/etc.).
        // Probe once; the lines feed both the shim check and the fallback below.
        let where_lines: Vec<String> = {
            let (ok, out) = crate::platform::run("where", &["claude"]);
            if ok {
                out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
            } else {
                Vec::new()
            }
        };
        for line in &where_lines {
            let pb = PathBuf::from(line);
            if pb.is_file() { return Some(pb); }
        }
        // TODO(verify): these are the common per-user install layouts on Windows.
        let candidates = [
            appdata().join(r"npm\claude.cmd"),
            appdata().join(r"npm\claude.exe"),
            localappdata().join(r"npm\claude.cmd"),
            userprofile().join(r".bun\bin\claude.exe"),
            userprofile().join(r".local\bin\claude.exe"),
            userprofile().join(r".claude\local\claude.exe"),
        ];
        if let Some(p) = candidates.into_iter().find(|p| p.is_file()) {
            return Some(p);
        }
        // Last resort (Windows field report): the shim may be present but its own
        // wrapper broken, or `where claude` may have missed because the GUI has no
        // npm bin on PATH. Probe the native exe the package unpacks.
        // TODO(verify): the exact nested npm layout (…/claude-code/node_modules/
        // @anthropic-ai/claude-code-win32-x64/claude.exe) is from one field report.
        let mut npm_dirs: Vec<PathBuf> = vec![appdata().join("npm")];
        for line in &where_lines {
            if let Some(dir) = PathBuf::from(line).parent() {
                npm_dirs.push(dir.to_path_buf());
            }
        }
        for dir in npm_dirs {
            let exe = super::nested_win_claude_exe(&dir);
            if exe.is_file() { return Some(exe); }
        }
        None
    }

    pub fn claude_bin_string() -> String {
        claude_bin().map(|p| p.display().to_string()).unwrap_or_else(|| "claude".into())
    }

    pub fn write_launcher(name: &str, slug: &str) -> Result<(), String> {
        fs::create_dir_all(launcher_root()).map_err(|e| e.to_string())?;
        let cfg = config_dir(slug).display().to_string();
        let body = render_launcher_win(name, &cfg, &claude_bin_string());
        fs::write(launcher_path(slug), body).map_err(|e| e.to_string())?;
        fs::write(name_path(slug), name).map_err(|e| e.to_string())?; // display-name sidecar
        Ok(())
    }

    pub fn write_login_script(name: &str, slug: &str) -> Result<PathBuf, String> {
        let dir = launcher_root().join("_login");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let cfg = config_dir(slug).display().to_string();
        let browser = crate::browser::prepare_login_browser(&dir);
        let body = render_login_win(name, &cfg, &claude_bin_string(), browser.as_deref());
        let p = login_path(slug);
        fs::write(&p, body).map_err(|e| e.to_string())?;
        Ok(p)
    }

    pub fn login_present(slug: &str) -> bool { credentials_present_at(&config_dir(slug)) }
    pub fn mark_onboarded(slug: &str) { mark_onboarded_at(&config_dir(slug)); }
}

/// Absolute path to claude if resolvable, else the bare command name "claude".
/// Used by ide.rs so a folderOpen task's `command` works regardless of the
/// login shell's PATH. Lives at the module root (not inside `mod imp`, which is
/// macOS-only) so it compiles on every target.
#[cfg(target_os = "macos")]
pub fn imp_claude_bin_string() -> String {
    imp::claude_bin().map(|p| p.display().to_string()).unwrap_or_else(|| "claude".to_string())
}
#[cfg(windows)]
pub fn imp_claude_bin_string() -> String { imp_win::claude_bin_string() }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn imp_claude_bin_string() -> String { "claude".to_string() }

/// This account engine's per-OS settings root (same dir `workspaces.json` lives
/// in). Exposed so `terminal.rs` can put `terminal-settings.json` next to it
/// without duplicating the path logic.
#[cfg(target_os = "macos")]
pub(crate) fn config_root() -> PathBuf { imp::config_root() }
#[cfg(windows)]
pub(crate) fn config_root() -> PathBuf { imp_win::config_root() }
#[cfg(not(any(target_os = "macos", windows)))]
pub(crate) fn config_root() -> PathBuf { std::env::temp_dir().join("ClaudeProfilesCLI") }

/// Runs the legacy `ClaudeProfilesCLI` → `Personae` folder migration (see the
/// per-OS doc comments on `migrate_legacy_dirs` in `imp`/`imp_win`). Exposed so
/// `ide.rs` can call it defensively too — `list_workspaces` reads from the same
/// root and could otherwise race `cli::list()` on startup (whichever the
/// frontend happens to invoke first). Idempotent, so calling it from both is
/// cheap and safe.
#[cfg(target_os = "macos")]
pub(crate) fn migrate_legacy_dirs() { imp::migrate_legacy_dirs() }
#[cfg(windows)]
pub(crate) fn migrate_legacy_dirs() { imp_win::migrate_legacy_dirs() }
#[cfg(not(any(target_os = "macos", windows)))]
pub(crate) fn migrate_legacy_dirs() {}

const LAUNCH_HISTORY_CAP: usize = 5;

fn launch_history_file() -> PathBuf { config_root().join("launch-history.json") }

fn load_launch_history_at(f: &Path) -> std::collections::HashMap<String, Vec<String>> {
    std::fs::read_to_string(f).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_launch_history_at(f: &Path, h: &std::collections::HashMap<String, Vec<String>>) -> Result<(), String> {
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let body = serde_json::to_string_pretty(h).map_err(|e| e.to_string())?;
    std::fs::write(f, body).map_err(|e| e.to_string())
}

/// Most-recent-first, deduped, capped at `LAUNCH_HISTORY_CAP`. Pure so it's
/// independently testable from the file I/O around it.
fn push_launch_location(mut list: Vec<String>, path: &str) -> Vec<String> {
    list.retain(|p| p != path);
    list.insert(0, path.to_string());
    list.truncate(LAUNCH_HISTORY_CAP);
    list
}

/// Recent project folders `launch(name, Some(path))` opened this account at —
/// most-recent first, capped at 5. Empty (not an error) for an unknown/never-
/// launched-with-a-folder account.
pub fn get_launch_history(name: &str) -> Vec<String> {
    let slug = resolve_slug(name);
    load_launch_history_at(&launch_history_file()).remove(&slug).unwrap_or_default()
}

/// Best-effort: a history-recording failure must never fail the launch itself.
fn record_launch_location(slug: &str, path: &str) {
    let f = launch_history_file();
    let mut h = load_launch_history_at(&f);
    let updated = push_launch_location(h.remove(slug).unwrap_or_default(), path);
    h.insert(slug.to_string(), updated);
    let _ = save_launch_history_at(&f, &h);
}

/// A plain `launch(name, Some(project_path))` (terminal only, no IDE) is a
/// real project↔account binding too — it was only feeding the launch-history
/// picker (`record_launch_location`, the "top 5 recent" list) and not the
/// Workspaces list on the account's own detail page, so a project launched
/// this way silently didn't show up there. `ide_id: "terminal"` distinguishes
/// this from a real `open_in_ide`-created binding (`workspace_id` includes
/// `ide_id`, so the two never collide even for the same account+path).
fn sync_launch_workspace(slug: &str, name: &str, path: &str) {
    record_launch_location(slug, path);
    let now = crate::platform::to_secs(std::time::SystemTime::now()).unwrap_or(0);
    let _ = crate::ide::save_workspace(slug, name, "terminal", "Terminal", path, now);
}

/// If this profile has a stored login, ensure its config dir is marked onboarded
/// so interactive `claude` (via Launch or an IDE terminal) skips the first-run
/// login menu and goes straight in on the stored account. No-op when not logged
/// in (we WANT the menu then, so the user can sign in). Called by cli::launch,
/// cli::list, and ide::open_in_ide.
#[cfg(target_os = "macos")]
pub fn ensure_onboarded(name: &str) {
    let slug = crate::platform::slugify(name);
    if imp::login_present(&slug) { imp::mark_onboarded(&slug); }
}
#[cfg(windows)]
pub fn ensure_onboarded(name: &str) {
    let slug = imp_win::resolve_slug(name);
    if imp_win::login_present(&slug) { imp_win::mark_onboarded(&slug); }
}
#[cfg(not(any(target_os = "macos", windows)))]
pub fn ensure_onboarded(_name: &str) {}

// ---- public API (macOS) --------------------------------------------------
#[cfg(target_os = "macos")]
pub fn available() -> bool { imp::claude_bin().is_some() }

#[cfg(target_os = "macos")]
pub fn create(name: &str) -> Result<(), String> {
    use crate::platform::slugify;
    let slug = slugify(name);
    if slug.is_empty() { return Err("Name must contain letters or numbers.".into()); }
    let lp = imp::launcher_path(name);
    if lp.exists() { return Err(format!("CLI profile already exists: {}", lp.display())); }
    std::fs::create_dir_all(imp::config_dir(&slug)).map_err(|e| e.to_string())?;
    imp::write_launcher(name, &slug)?;
    Ok(())
}

/// Renames a CLI account. The launcher `.command` file IS the display name
/// here (no separate name sidecar the way Windows has), and `slug` — which
/// `config_dir()` and the Keychain-service hash are both derived from — is
/// recomputed from the name rather than stored, so a rename that changes the
/// slugified form has to move the config dir *and* the Keychain login entry
/// to the new hash, or the account would silently appear signed-out
/// afterward. Validates everything, including reading back the Keychain
/// entry, before making any filesystem change — a failure at any step
/// leaves the account exactly as it was, nothing partially renamed.
///
/// UNVERIFIED on a real Mac this session (no access to one) — the Keychain
/// read/re-add specifically (`security ... -w`) needs a real pass before
/// trusting it against a real login.
#[cfg(target_os = "macos")]
pub fn rename(old_name: &str, new_name: &str) -> Result<(), String> {
    use crate::platform::slugify;
    let old_slug = slugify(old_name);
    let old_lp = imp::launcher_path(old_name);
    if !old_lp.exists() { return Err(format!("No such CLI account: {old_name}")); }

    let new_name = new_name.trim();
    let new_slug = slugify(new_name);
    if new_slug.is_empty() { return Err("Name must contain letters or numbers.".into()); }
    let new_lp = imp::launcher_path(new_name);
    if new_lp != old_lp && new_lp.exists() {
        return Err(format!("CLI account already exists: {new_name}"));
    }

    if new_slug != old_slug {
        let old_cfg = imp::config_dir(&old_slug);
        let new_cfg = imp::config_dir(&new_slug);
        if new_cfg.exists() {
            return Err(format!("An account already uses the internal id \"{new_slug}\" — pick a more distinct name."));
        }

        // Keychain has no "rename service" primitive for generic passwords —
        // read the secret back and re-add it under the new hash, then drop
        // the old entry. Bail before touching anything on disk if this step
        // can't be completed cleanly.
        let old_svc = imp::login_keychain_service(&old_slug);
        let new_svc = imp::login_keychain_service(&new_slug);
        let user = std::env::var("USER").unwrap_or_default();
        if crate::platform::run("security", &["find-generic-password", "-s", &old_svc, "-a", &user]).0 {
            let (read_ok, pw) = crate::platform::run("security", &["find-generic-password", "-s", &old_svc, "-a", &user, "-w"]);
            let pw = pw.trim().to_string();
            if !read_ok || pw.is_empty() {
                return Err("Could not read this account's saved login to move it — rename aborted, nothing was changed.".into());
            }
            let (added, _) = crate::platform::run("security", &["add-generic-password", "-s", &new_svc, "-a", &user, "-w", &pw]);
            if !added {
                return Err("Could not move this account's saved login to the new name — rename aborted, nothing was changed.".into());
            }
            let _ = crate::platform::run("security", &["delete-generic-password", "-s", &old_svc, "-a", &user]);
        }

        if old_cfg.exists() {
            std::fs::rename(&old_cfg, &new_cfg).map_err(|e| e.to_string())?;
        }
    }

    if old_lp != new_lp {
        let _ = std::fs::remove_file(&old_lp);
    }
    imp::write_launcher(new_name, &new_slug)
}

/// Open a Terminal in this profile's config dir running `claude auth login` — the
/// one-time real sign-in. The credential is stored in this profile's own
/// Keychain slot (namespaced by config dir), isolated from other profiles.
#[cfg(target_os = "macos")]
pub fn login(name: &str) -> Result<(), String> {
    use crate::platform::slugify;
    let slug = slugify(name);
    if !imp::launcher_path(name).exists() { return Err(format!("No such CLI profile: {name}")); }
    let bin = imp::claude_bin().ok_or("Claude Code CLI not found (install `claude`).")?;
    let cfg = imp::config_dir(&slug);
    let shell_cmd = format!(
        "export CLAUDE_CONFIG_DIR={}; {} auth login",
        sh_quote(&cfg.display().to_string()), sh_quote(&bin.display().to_string())
    );
    let as_arg = shell_cmd.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("tell application \"Terminal\" to do script \"{}\"", as_arg);
    let (ok, out) = crate::platform::run("osascript", &["-e", &script, "-e", "tell application \"Terminal\" to activate"]);
    if ok { Ok(()) } else { Err(out.trim().to_string()) }
}

#[cfg(target_os = "macos")]
pub fn launch(name: &str, project_path: Option<&str>) -> Result<(), String> {
    let lp = imp::launcher_path(name);
    if !lp.exists() { return Err(format!("No such CLI profile: {name}")); }
    ensure_onboarded(name); // skip the first-run menu if already logged in
    // `open` hands off to Terminal.app and exits on its own — unlike the
    // Windows `cmd /C start` case, it isn't known to hang on Command::output().
    // NOT wired to actually cd into project_path (unverified on macOS this
    // session): `open <script>` starts Terminal.app at the script's own
    // directory via LaunchServices, not the invoking process's cwd, so a
    // Windows-style current_dir() fix wouldn't take effect here the same way.
    let (ok, e) = crate::platform::run("open", &[&lp.display().to_string()]);
    if !ok { return Err(e); }
    if let Some(p) = project_path {
        sync_launch_workspace(&crate::platform::slugify(name), name, p);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn list() -> Vec<CliProfile> {
    use crate::platform::{run, slugify, to_secs};
    imp::migrate_legacy_dirs();
    let mut out = vec![];
    let rd = match std::fs::read_dir(imp::launcher_root()) { Ok(r) => r, Err(_) => return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map_or(false, |x| x == "command") {
            let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let slug = slugify(&name);
            let cfg = imp::config_dir(&slug);
            // Self-heal the launcher script on every list — see the matching
            // Windows list()'s comment for why this is safe/idempotent.
            let _ = imp::write_launcher(&name, &slug);
            let data_size = if cfg.exists() {
                let (_, o) = run("du", &["-sh", &cfg.display().to_string()]);
                o.split('\t').next().unwrap_or("—").trim().to_string()
            } else { "—".into() };
            let claude_json = std::fs::read_to_string(cfg.join(".claude.json")).ok();
            let account_email = claude_json.as_deref().and_then(extract_email);
            let (session_usage_pct, weekly_usage_pct) = claude_json.as_deref()
                .map(extract_usage_utilization).unwrap_or((None, None));
            let provider = load_provider_config(&cfg);
            // Reliable signal = the login's Keychain slot exists (email is only a
            // lazily-cached display value that may lag behind the actual login) —
            // OR an api-key provider is configured, which needs no OAuth at all.
            let oauth_logged_in = imp::login_present(&slug);
            if oauth_logged_in { imp::mark_onboarded(&slug); } // self-heal: skip first-run menu once logged in
            let logged_in = oauth_logged_in || provider.auth_mode == "api_key";
            let keychain_creds = if oauth_logged_in { imp::read_login_credentials(&slug) } else { None };
            let (token_expires_at, refresh_expires_at) = keychain_creds.as_deref()
                .map(extract_token_expiry).unwrap_or((None, None));
            let (subscription_type, rate_limit_tier) = keychain_creds.as_deref()
                .map(extract_plan_tier).unwrap_or((None, None));
            let md = std::fs::metadata(&cfg).ok();
            let created = md.as_ref().and_then(|m| m.created().ok()).and_then(to_secs);
            let last_active = md.as_ref().and_then(|m| m.modified().ok()).and_then(to_secs);
            out.push(CliProfile {
                name, slug,
                config_dir: cfg.display().to_string(),
                launcher_path: p.display().to_string(),
                logged_in, account_email, data_size, created, last_active,
                auth_mode: provider.auth_mode, provider_model: provider.model,
                token_expires_at, refresh_expires_at,
                subscription_type, rate_limit_tier,
                session_usage_pct, weekly_usage_pct,
            });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

#[cfg(target_os = "macos")]
pub fn delete(name: &str, purge: bool) -> Result<(), String> {
    use crate::platform::slugify;
    let slug = slugify(name);
    let lp = imp::launcher_path(name);
    if !lp.exists() { return Err(format!("No such CLI profile: {name}")); }
    std::fs::remove_file(&lp).map_err(|e| e.to_string())?;
    if purge {
        // remove claude's per-profile login from the Keychain, then the data dir
        let svc = imp::login_keychain_service(&slug);
        let user = std::env::var("USER").unwrap_or_default();
        let _ = crate::platform::run("security", &["delete-generic-password", "-s", &svc, "-a", &user]);
        let cfg = imp::config_dir(&slug);
        if cfg.exists() { std::fs::remove_dir_all(&cfg).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

// ---- public API (Windows) ------------------------------------------------
// Isolation is by CLAUDE_CONFIG_DIR alone (see imp_win header). Every process /
// quoting assumption below is marked TODO(verify) for the real-Windows pass.
#[cfg(windows)]
pub fn available() -> bool { imp_win::claude_bin().is_some() }

#[cfg(windows)]
pub fn create(name: &str) -> Result<(), String> {
    use crate::platform::slugify;
    let slug = slugify(name);
    if slug.is_empty() { return Err("Name must contain letters or numbers.".into()); }
    if imp_win::launcher_path(&slug).exists() { return Err(format!("CLI account already exists: {name}")); }
    std::fs::create_dir_all(imp_win::config_dir(&slug)).map_err(|e| e.to_string())?;
    imp_win::write_launcher(name, &slug)?;
    Ok(())
}

/// Rename only the *display* name — `slug`/`config_dir` (and everything
/// keyed by it: credentials, launch history, workspace bindings) are
/// untouched, since Windows already decouples them: the `.name` sidecar next
/// to the launcher is the only place the display name lives, independent of
/// the slug-named `.cmd`/config dir. Regenerates the launcher (`.cmd` +
/// `.name`) via `write_launcher` so the new name is reflected in the
/// console-title/banner lines it bakes in (see `render_launcher_win`), not
/// just the sidecar file.
#[cfg(windows)]
pub fn rename(old_name: &str, new_name: &str) -> Result<(), String> {
    let slug = imp_win::resolve_slug(old_name);
    if !imp_win::launcher_path(&slug).exists() { return Err(format!("No such CLI account: {old_name}")); }
    let new_name = new_name.trim();
    if new_name.is_empty() { return Err("Name must contain letters or numbers.".into()); }
    imp_win::write_launcher(new_name, &slug)
}

/// A console launched via `cmd /C start ...` otherwise inherits Personae's own
/// process cwd (wherever the app happened to be started from — not somewhere a
/// user can usefully `cd` around from), so every Windows console this module
/// opens starts at the user's home folder instead.
#[cfg(windows)]
fn user_home_dir() -> PathBuf {
    std::env::var("USERPROFILE").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("C:\\"))
}

/// Open a one-shot console running `claude auth login` for this account's
/// CLAUDE_CONFIG_DIR — the real sign-in. On Windows the credential is written to
/// <config_dir>\.credentials.json (Keychain is absent), isolated per account.
#[cfg(windows)]
pub fn login(name: &str) -> Result<(), String> {
    let slug = imp_win::resolve_slug(name);
    if !imp_win::launcher_path(&slug).exists() { return Err(format!("No such CLI account: {name}")); }
    imp_win::claude_bin().ok_or("Claude Code CLI not found (install `claude`).")?;
    let script = imp_win::write_login_script(name, &slug)?;
    // Open the one-shot login console in its own window. TODO(verify) start quoting:
    // the empty "" is the (unused) window title so a quoted script path is not
    // mistaken for the title. spawn_detached_in, not run_in: this opens a
    // long-lived interactive console — see that helper's doc comment.
    crate::platform::spawn_detached_in(
        "cmd", &["/C", "start", "", &script.display().to_string()], &user_home_dir(),
    )
}

#[cfg(windows)]
pub fn launch(name: &str, project_path: Option<&str>) -> Result<(), String> {
    let slug = imp_win::resolve_slug(name);
    let lp = imp_win::launcher_path(&slug);
    if !lp.exists() { return Err(format!("No such CLI account: {name}")); }
    ensure_onboarded(name); // skip the first-run menu if already logged in
    // New interactive console running the launcher (keeps window; user gets a
    // claude REPL). Pass the launcher path BARE (no manual quotes):
    // std::process::Command already quotes each arg, so a pre-quoted string
    // would reach the shell as literal quotes. Which terminal opens it is the
    // user's saved preference (terminal.rs); default/unset/removed picks fall
    // through to today's bare `cmd`. cwd is the chosen project folder if any,
    // else the user's home — either way, never Personae's own launch dir.
    let lp_str = lp.display().to_string();
    let cwd = match project_path {
        Some(p) => PathBuf::from(p),
        None => user_home_dir(),
    };
    // spawn_detached_in, not run_in: this opens a long-lived interactive
    // console — see that helper's doc comment (confirmed live: the equivalent
    // Desktop-launch call hung 60+s under `run`/`run_in`'s Command::output()).
    let result = match crate::terminal::get_default_terminal().as_deref() {
        Some("powershell") => crate::platform::spawn_detached_in(
            "cmd",
            &["/C", "start", "", "powershell", "-NoExit", "-Command",
              &format!("& '{}'", lp_str.replace('\'', "''"))],
            &cwd,
        ),
        Some("windows-terminal") if crate::terminal::windows_terminal_available() => crate::platform::spawn_detached_in(
            "wt.exe",
            &["cmd", "/K", &lp_str],
            &cwd,
        ),
        Some("custom") => match crate::terminal::get_custom_terminal_path() {
            // Best-effort: pass the launcher path as a bare argument. Works for
            // terminals that accept "open here and run me" positionally (matches
            // how the cmd/wt branches above invoke the launcher); not every
            // third-party terminal's CLI accepts this — there's no universal
            // convention across terminal emulators.
            Some(custom) => crate::platform::spawn_detached_in(&custom, &[&lp_str], &cwd),
            None => crate::platform::spawn_detached_in("cmd", &["/C", "start", "", "cmd", "/K", &lp_str], &cwd),
        },
        _ => crate::platform::spawn_detached_in(
            "cmd",
            &["/C", "start", "", "cmd", "/K", &lp_str],
            &cwd,
        ),
    };
    if result.is_ok() {
        if let Some(p) = project_path {
            sync_launch_workspace(&slug, name, p);
        }
    }
    result
}

#[cfg(windows)]
pub fn list() -> Vec<CliProfile> {
    use crate::platform::to_secs;
    imp_win::migrate_legacy_dirs();
    let mut out = vec![];
    let rd = match std::fs::read_dir(imp_win::launcher_root()) { Ok(r) => r, Err(_) => return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map_or(false, |x| x == "cmd") {
            let slug = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let name = std::fs::read_to_string(imp_win::name_path(&slug)).ok()
                .map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
                .unwrap_or_else(|| slug.clone());
            let cfg = imp_win::config_dir(&slug);
            // Self-heal the launcher script on every list, same spirit as the
            // mark_onboarded self-heal below: `write_launcher` is a pure function
            // of (name, slug) and deterministically regenerates identical content
            // for an unchanged account, so this is safe/idempotent — but it means
            // an *existing* account's launcher always reflects the latest
            // generator code (e.g. the CLAUDE_CODE_CHILD_SESSION clear-line) after
            // an app update, not just newly-created accounts.
            let _ = imp_win::write_launcher(&name, &slug);
            let data_size = if cfg.exists() { human_size(dir_size_bytes(&cfg)) } else { "—".into() };
            let claude_json = std::fs::read_to_string(cfg.join(".claude.json")).ok();
            let account_email = claude_json.as_deref().and_then(extract_email);
            let (session_usage_pct, weekly_usage_pct) = claude_json.as_deref()
                .map(extract_usage_utilization).unwrap_or((None, None));
            let credentials_json = std::fs::read_to_string(cfg.join(".credentials.json")).ok();
            let (token_expires_at, refresh_expires_at) = credentials_json.as_deref()
                .map(extract_token_expiry).unwrap_or((None, None));
            let (subscription_type, rate_limit_tier) = credentials_json.as_deref()
                .map(extract_plan_tier).unwrap_or((None, None));
            let provider = load_provider_config(&cfg);
            let oauth_logged_in = imp_win::login_present(&slug);
            if oauth_logged_in { imp_win::mark_onboarded(&slug); } // self-heal: skip first-run menu
            let logged_in = oauth_logged_in || provider.auth_mode == "api_key";
            let md = std::fs::metadata(&cfg).ok();
            let created = md.as_ref().and_then(|m| m.created().ok()).and_then(to_secs);
            let last_active = md.as_ref().and_then(|m| m.modified().ok()).and_then(to_secs);
            out.push(CliProfile {
                name, slug,
                config_dir: cfg.display().to_string(),
                launcher_path: p.display().to_string(),
                logged_in, account_email, data_size, created, last_active,
                auth_mode: provider.auth_mode, provider_model: provider.model,
                token_expires_at, refresh_expires_at,
                subscription_type, rate_limit_tier,
                session_usage_pct, weekly_usage_pct,
            });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

#[cfg(windows)]
pub fn delete(name: &str, purge: bool) -> Result<(), String> {
    let slug = imp_win::resolve_slug(name);
    let lp = imp_win::launcher_path(&slug);
    if !lp.exists() { return Err(format!("No such CLI account: {name}")); }
    std::fs::remove_file(&lp).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(imp_win::name_path(&slug));
    let _ = std::fs::remove_file(imp_win::login_path(&slug));
    if purge {
        // No Keychain on Windows: the login lives in <config_dir>\.credentials.json,
        // which is removed with the dir.
        let cfg = imp_win::config_dir(&slug);
        if cfg.exists() { std::fs::remove_dir_all(&cfg).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

// ---- public API (Linux/other stubs; not yet implemented) -----------------
#[cfg(not(any(target_os = "macos", windows)))]
const NOT_YET: &str = "CLI accounts are macOS/Windows-only for now.";
#[cfg(not(any(target_os = "macos", windows)))]
pub fn available() -> bool { false }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn list() -> Vec<CliProfile> { vec![] }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn create(_name: &str) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn rename(_old_name: &str, _new_name: &str) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn login(_name: &str) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn launch(_name: &str, _project_path: Option<&str>) -> Result<(), String> { Err(NOT_YET.into()) }
#[cfg(not(any(target_os = "macos", windows)))]
pub fn delete(_name: &str, _purge: bool) -> Result<(), String> { Err(NOT_YET.into()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_sets_config_dir_and_execs() {
        let s = render_launcher("Work", "/tmp/cfg dir", "/opt/homebrew/bin/claude");
        assert!(s.contains("export CLAUDE_CONFIG_DIR=\"/tmp/cfg dir\""));
        assert!(s.trim_end().ends_with("exec \"$CLAUDE_BIN\" \"$@\""));
        assert!(!s.contains("security"));                // no keychain reads
        assert!(!s.contains("CLAUDE_CODE_OAUTH_TOKEN"));  // no token injection
    }

    #[test]
    fn sh_dq_escape_neutralizes_metachars() {
        assert_eq!(sh_dq_escape("a\"$`\\b"), "a\\\"\\$\\`\\\\b");
    }

    #[test]
    fn launcher_escapes_quotes_in_name() {
        let s = render_launcher("Ac\"me", "/tmp/x", "/bin/claude");
        assert!(s.contains("Ac\\\"me"));
    }

    #[test]
    fn sh_quote_escapes_single_quotes() {
        assert_eq!(sh_quote("/tmp/plain"), "'/tmp/plain'");
        assert_eq!(sh_quote("/Users/O'Brien/x"), "'/Users/O'\\''Brien/x'");
    }

    #[test]
    fn keychain_service_matches_claude_derivation() {
        // Verified empirically: real /login in this dir created Keychain item
        // "Claude Code-credentials-5f560a22".
        assert_eq!(
            login_keychain_service_for_dir("/Users/amoryzenith/Library/Application Support/ClaudeProfilesCLI/proto1"),
            "Claude Code-credentials-5f560a22"
        );
    }

    #[test]
    fn extract_usage_utilization_reads_session_and_weekly_all() {
        // Shape verified live against a real account's .claude.json this session.
        let j = r#"{"cachedUsageUtilization":{"utilization":{"five_hour":{"utilization":29},"limits":[
            {"kind":"session","group":"session","percent":29},
            {"kind":"weekly_all","group":"weekly","percent":7},
            {"kind":"weekly_scoped","group":"weekly","percent":2,"scope":{"model":{"display_name":"Fable"}}}
        ]}}}"#;
        assert_eq!(extract_usage_utilization(j), (Some(29), Some(7)));
        assert_eq!(extract_usage_utilization(r#"{"no":"usage"}"#), (None, None));
        assert_eq!(extract_usage_utilization("not json"), (None, None));
        let jf = r#"{"cachedUsageUtilization":{"utilization":{"five_hour":{"utilization":29.6},"limits":[
            {"kind":"weekly_all","percent":7.4}
        ]}}}"#;
        assert_eq!(extract_usage_utilization(jf), (Some(30), Some(7)));
    }

    #[test]
    fn parse_utilization_percentages_rounds_live_api_floats() {
        // Shape verified live: a real GET https://api.anthropic.com/api/oauth/usage
        // response body IS this "utilization" object directly (no wrapping
        // cachedUsageUtilization key), and its numbers are explicit floats
        // (e.g. "2.0"), unlike the CLI's own cache which serializes them bare.
        let body: serde_json::Value = serde_json::from_str(
            r#"{"five_hour":{"utilization":2.0},"limits":[
                {"kind":"weekly_all","group":"weekly","percent":9.0}
            ]}"#
        ).unwrap();
        assert_eq!(parse_utilization_percentages(Some(&body)), (Some(2), Some(9)));
        assert_eq!(parse_utilization_percentages(None), (None, None));
    }

    #[test]
    fn extract_email_works() {
        let j = r#"{"userID":"x","oauthAccount":{"emailAddress":"a@b.com","org":"y"}}"#;
        assert_eq!(extract_email(j).as_deref(), Some("a@b.com"));
        assert_eq!(extract_email(r#"{"no":"email"}"#), None);
    }

    #[test]
    fn extract_token_expiry_reads_credentials_json_shape() {
        // Shape verified live against a real Windows .credentials.json this session
        // (field names/nesting only — values below are placeholders, not real tokens).
        let j = r#"{"claudeAiOauth":{"accessToken":"x","refreshToken":"y","expiresAt":1787089852342,"refreshTokenExpiresAt":1789315069342,"scopes":[]}}"#;
        assert_eq!(extract_token_expiry(j), (Some(1787089852342), Some(1789315069342)));
        assert_eq!(extract_token_expiry(r#"{"no":"oauth"}"#), (None, None));
        assert_eq!(extract_token_expiry("not json"), (None, None));
    }

    #[test]
    fn extract_plan_tier_reads_subscription_and_rate_limit() {
        // Shape verified live against a real Windows .credentials.json this session.
        let j = r#"{"claudeAiOauth":{"subscriptionType":"max","rateLimitTier":"default_claude_max_20x"}}"#;
        assert_eq!(extract_plan_tier(j), (Some("max".into()), Some("default_claude_max_20x".into())));
        // api-key-mode accounts have no claudeAiOauth object at all.
        assert_eq!(extract_plan_tier(r#"{"no":"oauth"}"#), (None, None));
    }

    #[test]
    fn push_launch_location_dedupes_orders_and_caps() {
        let list = push_launch_location(vec![], "/a");
        let list = push_launch_location(list, "/b");
        let list = push_launch_location(list, "/c");
        assert_eq!(list, vec!["/c", "/b", "/a"]);

        // re-pushing an existing entry moves it to the front instead of duplicating
        let list = push_launch_location(list, "/a");
        assert_eq!(list, vec!["/a", "/c", "/b"]);

        let list = push_launch_location(list, "/d");
        let list = push_launch_location(list, "/e");
        let list = push_launch_location(list, "/f"); // 6th entry — must evict the oldest
        assert_eq!(list.len(), 5);
        assert_eq!(list[0], "/f");
        assert!(!list.contains(&"/b".to_string()), "oldest entry should have been evicted: {list:?}");
    }

    #[test]
    fn launch_history_roundtrips_per_account_and_ignores_unknown() {
        let f = std::env::temp_dir().join(format!("launch-history-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);

        let mut h = load_launch_history_at(&f);
        assert!(h.is_empty());
        h.insert("work".into(), push_launch_location(vec![], "/projects/a"));
        save_launch_history_at(&f, &h).unwrap();

        let loaded = load_launch_history_at(&f);
        assert_eq!(loaded.get("work"), Some(&vec!["/projects/a".to_string()]));
        assert_eq!(loaded.get("unknown-account"), None);

        std::fs::remove_file(&f).ok();
    }

    // ---- Windows pure-helper tests (run on macOS) ------------------------

    #[test]
    fn win_launcher_sets_config_dir_and_calls_bin() {
        let s = render_launcher_win("Work", r"C:\Users\me\AppData\Roaming\ClaudeProfilesCLI\work", r"C:\Users\me\AppData\Roaming\npm\claude.cmd");
        assert!(s.starts_with("@echo off\r\n"));
        assert!(s.contains("set \"CLAUDE_CONFIG_DIR=C:\\Users\\me\\AppData\\Roaming\\ClaudeProfilesCLI\\work\""));
        assert!(s.contains("set \"CLAUDE_BIN=C:\\Users\\me\\AppData\\Roaming\\npm\\claude.cmd\""));
        assert!(s.contains("if not exist \"%CLAUDE_BIN%\" set \"CLAUDE_BIN=claude\""));
        assert!(s.contains("call \"%CLAUDE_BIN%\" %*"));
        assert!(s.ends_with("\r\n"));
    }

    #[test]
    fn win_value_escape_doubles_percent() {
        assert_eq!(cmd_set_value_escape("a%b%c"), "a%%b%%c");
        assert_eq!(cmd_set_value_escape(r"C:\plain\path"), r"C:\plain\path");
    }

    #[test]
    fn win_login_script_runs_auth_login_and_pauses() {
        let s = render_login_win("Work", r"C:\cfg\work", r"C:\bin\claude.cmd", None);
        assert!(s.contains("set \"CLAUDE_CONFIG_DIR=C:\\cfg\\work\""));
        assert!(s.contains("set \"CLAUDE_BIN=C:\\bin\\claude.cmd\""));
        assert!(s.contains("call \"%CLAUDE_BIN%\" auth login"));
        assert!(s.contains("pause"));
        assert!(!s.contains("BROWSER"));
    }

    #[test]
    fn win_login_script_sets_browser_when_provided() {
        let s = render_login_win("Work", r"C:\cfg\work", r"C:\bin\claude.cmd", Some(r"C:\browsers\chrome.exe"));
        assert!(s.contains("set \"BROWSER=C:\\browsers\\chrome.exe\""));
        // BROWSER must be set before the auth login call actually reads it.
        assert!(s.find("BROWSER=").unwrap() < s.find("auth login").unwrap());
    }

    #[test]
    fn credentials_present_detects_file() {
        let d = std::env::temp_dir().join(format!("cli-cred-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert!(!credentials_present_at(&d));
        std::fs::write(d.join(".credentials.json"), r#"{"claudeAiOauth":{}}"#).unwrap();
        assert!(credentials_present_at(&d));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn mark_onboarded_sets_flag_idempotently() {
        let d = std::env::temp_dir().join(format!("cli-onb-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        mark_onboarded_at(&d);
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(d.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(v["hasCompletedOnboarding"], serde_json::json!(true));
        // Second call is a no-op (flag already true) and must not error.
        mark_onboarded_at(&d);
        let v2: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(d.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(v2["hasCompletedOnboarding"], serde_json::json!(true));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn win_launcher_comment_is_single_line_and_escapes_percent() {
        // The display name lands ONLY in the REM comment. CR/LF must be stripped
        // so an injected `set EVIL=1` cannot break out into its own command line,
        // and `%` must be doubled so it isn't expanded.
        let s = render_launcher_win("a\r\nset EVIL=1\r\nrem %x%", r"C:\cfg\a", r"C:\bin\claude.cmd");
        let rem_lines: Vec<&str> = s.lines().filter(|l| l.trim_start().starts_with("REM")).collect();
        assert_eq!(rem_lines.len(), 1, "name must not inject extra lines");
        let rem = rem_lines[0];
        assert!(rem.contains("aset EVIL=1rem"), "CR/LF should be stripped, got: {rem}");
        assert!(rem.contains("%%x%%"), "percent must be doubled in the comment");
        // The injected fragment must never appear as its own command line.
        assert!(!s.lines().any(|l| l.trim() == "set EVIL=1"));
    }

    #[test]
    fn mark_onboarded_preserves_existing_json() {
        let d = std::env::temp_dir().join(format!("cli-onb-preserve-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join(".claude.json"),
            r#"{"oauthAccount":{"emailAddress":"a@b.com"},"foo":"bar"}"#,
        ).unwrap();
        mark_onboarded_at(&d);
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(d.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(v["hasCompletedOnboarding"], serde_json::json!(true));
        assert_eq!(v["foo"], serde_json::json!("bar"));
        assert_eq!(v["oauthAccount"]["emailAddress"], serde_json::json!("a@b.com"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn nested_win_exe_path_is_assembled() {
        // Given the npm global bin dir (where the `claude` shim lives), we derive
        // the native exe the claude-code package unpacks for win32-x64.
        let base = Path::new("NPMROOT");
        let got = nested_win_claude_exe(base);
        let s = got.to_string_lossy();
        assert!(got.ends_with("claude.exe"), "must end at claude.exe: {s}");
        assert!(s.contains("@anthropic-ai"), "must nest under @anthropic-ai: {s}");
        assert!(s.contains("claude-code-win32-x64"), "must hit the win32-x64 pkg: {s}");
        // The nesting is claude-code -> node_modules -> claude-code-win32-x64, so
        // exactly two `node_modules` segments appear.
        assert_eq!(s.matches("node_modules").count(), 2, "exactly two node_modules: {s}");
    }
}
