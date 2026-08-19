import type { BrowserApp, BrowserPrefs, CliProfile, DesktopProfile, IdeInfo, ProviderConfig, TerminalApp, Workspace } from "./types";

// ---------- Tauri bridge (with a mock fallback for browser preview) ----------
const hasTauri = !!(window.__TAURI__ && window.__TAURI__.core);

export const nowSecs = () => Math.floor(Date.now() / 1000);

const MOCK: DesktopProfile[] = [
  { name: "Personal", slug: "personal", tint: "#C2714F", running: false,
    data_size: "7.1M", data_path: "/Users/you/Library/Application Support/Claude-Personal",
    created: nowSecs() - 3600, last_active: nowSecs() - 12 },
  { name: "Work", slug: "work", tint: "#14B8A6", running: true,
    data_size: "128M", data_path: "/Users/you/Library/Application Support/Claude-Work",
    created: nowSecs() - 86400 * 9, last_active: nowSecs() - 4200 },
];

const MOCK_CLI: CliProfile[] = [
  { name: "Work CLI", slug: "work-cli", config_dir: "/Users/you/Library/Application Support/Personae/CLI/work-cli",
    launcher_path: "/Users/you/Applications/Personae/CLI/Work CLI.command",
    logged_in: true, account_email: "work@corp.com", data_size: "12M", created: nowSecs() - 86400, last_active: nowSecs() - 300,
    auth_mode: "oauth", provider_model: null,
    token_expires_at: (nowSecs() + 3600) * 1000, refresh_expires_at: (nowSecs() + 86400 * 60) * 1000,
    subscription_type: "max", rate_limit_tier: "default_claude_max_20x",
    session_usage_pct: 29, weekly_usage_pct: 7 },
  { name: "Personal CLI", slug: "personal-cli", config_dir: "/Users/you/Library/Application Support/Personae/CLI/personal-cli",
    launcher_path: "/Users/you/Applications/Personae/CLI/Personal CLI.command",
    logged_in: false, account_email: null, data_size: "0B", created: nowSecs() - 3600, last_active: null,
    auth_mode: "oauth", provider_model: null, token_expires_at: null, refresh_expires_at: null,
    subscription_type: null, rate_limit_tier: null, session_usage_pct: null, weekly_usage_pct: null },
];

const MOCK_WS: Workspace[] = [
  { id: "cursorwork-cli/Users/you/Projects/demo", account_slug: "work-cli", account_name: "Work CLI",
    ide_id: "cursor", ide_name: "Cursor", project_path: "/Users/you/Projects/demo",
    created: nowSecs() - 3600, last_opened: nowSecs() - 600 },
];

const MOCK_LAUNCH_HISTORY: Record<string, string[]> = {};
const MOCK_PROVIDERS: Record<string, ProviderConfig> = {};
const emptyProviderConfig = (): ProviderConfig => ({ auth_mode: "oauth", base_url: null, api_key: null, model: null, small_fast_model: null });

let MOCK_DEFAULT_TERMINAL: string | null = null;
let MOCK_CUSTOM_TERMINAL_PATH: string | null = null;
const MOCK_TERMINALS: TerminalApp[] = [
  { id: "cmd", name: "Command Prompt" },
  { id: "powershell", name: "PowerShell" },
  { id: "windows-terminal", name: "Windows Terminal" },
];

let MOCK_DESKTOP_EXE_OVERRIDE: string | null = null;

let MOCK_BROWSER_PREFS: BrowserPrefs = { browser_id: null, custom_path: null, reuse_profile: true };
const MOCK_BROWSERS: BrowserApp[] = [
  { id: "chrome", name: "Google Chrome" },
  { id: "edge", name: "Microsoft Edge" },
];

export async function invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (hasTauri) return window.__TAURI__!.core.invoke<T>(cmd, args);
  // ---- browser mock ----
  switch (cmd) {
    case "claude_found": return true as T;
    case "list_profiles": return structuredClone(MOCK) as T;
    case "create_profile": {
      const name = args!.name as string;
      const color = args!.color as string | undefined;
      const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
      MOCK.push({ name, slug, tint: color ? `#${color}` : "#8B5CF6",
        running: false, data_size: "0B",
        data_path: `/Users/you/Library/Application Support/Claude-${name}`,
        created: nowSecs(), last_active: nowSecs() });
      return undefined as T;
    }
    case "set_profile_color": { const p = MOCK.find(m => m.name === args!.name); if (p) p.tint = `#${args!.color}`; return undefined as T; }
    case "get_desktop_exe_override": return MOCK_DESKTOP_EXE_OVERRIDE as T;
    case "set_desktop_exe_override": MOCK_DESKTOP_EXE_OVERRIDE = (args!.path as string | null) ?? null; return undefined as T;
    case "pick_desktop_exe": return "C:\\Program Files\\AnthropicClaude\\Claude.exe" as T;
    case "launch_profile": { const p = MOCK.find(m => m.name === args!.name); if (p) p.running = true; return undefined as T; }
    case "quit_profile": { const p = MOCK.find(m => m.name === args!.name); if (p) p.running = false; return undefined as T; }
    case "delete_profile": { const i = MOCK.findIndex(m => m.name === args!.name); if (i >= 0) MOCK.splice(i, 1); return undefined as T; }
    case "repair_profiles": return MOCK.length as T;
    case "reveal_path": console.log("reveal", args!.path); return undefined as T;
    case "open_url": console.log("open", args!.url); return undefined as T;
    case "cli_available": return true as T;
    case "list_cli_profiles": return structuredClone(MOCK_CLI) as T;
    case "create_cli_profile": {
      const name = args!.name as string;
      const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
      MOCK_CLI.push({ name, slug, config_dir: `/Users/you/Library/Application Support/Personae/CLI/${slug}`,
        launcher_path: `/Users/you/Applications/Personae/CLI/${name}.command`,
        logged_in: false, account_email: null, data_size: "0B", created: nowSecs(), last_active: nowSecs(),
        auth_mode: "oauth", provider_model: null, token_expires_at: null, refresh_expires_at: null,
        subscription_type: null, rate_limit_tier: null, session_usage_pct: null, weekly_usage_pct: null });
      return undefined as T;
    }
    case "rename_cli_profile": {
      const oldName = args!.oldName as string;
      const newName = (args!.newName as string).trim();
      const p = MOCK_CLI.find((m) => m.name === oldName);
      if (!p) throw `No such CLI account: ${oldName}`;
      if (!newName) throw "Name must contain letters or numbers.";
      p.name = newName;
      return undefined as T;
    }
    case "login_cli_profile": console.log("login", args!.name); return undefined as T;
    case "launch_cli_profile": {
      const name = args!.name as string;
      const projectPath = args!.projectPath as string | null | undefined;
      console.log("launch cli", name, projectPath);
      if (projectPath) {
        const p = MOCK_CLI.find((m) => m.name === name);
        if (p) {
          const list = (MOCK_LAUNCH_HISTORY[p.slug] ??= []);
          const i = list.indexOf(projectPath);
          if (i >= 0) list.splice(i, 1);
          list.unshift(projectPath);
          list.length = Math.min(list.length, 5);
        }
      }
      return undefined as T;
    }
    case "get_launch_history": {
      const p = MOCK_CLI.find((m) => m.name === args!.name);
      return (p ? MOCK_LAUNCH_HISTORY[p.slug] ?? [] : []) as T;
    }
    case "fetch_live_cli_usage": {
      const p = MOCK_CLI.find((m) => m.name === args!.name);
      return (p ? [p.session_usage_pct, p.weekly_usage_pct] : [null, null]) as T;
    }
    case "delete_cli_profile": {
      const i = MOCK_CLI.findIndex(m => m.name === args!.name);
      if (i >= 0) { delete MOCK_PROVIDERS[MOCK_CLI[i].slug]; MOCK_CLI.splice(i, 1); }
      return undefined as T;
    }
    case "get_cli_provider_config": {
      const p = MOCK_CLI.find(m => m.name === args!.name);
      return (p ? MOCK_PROVIDERS[p.slug] ?? emptyProviderConfig() : emptyProviderConfig()) as T;
    }
    case "set_cli_provider_config": {
      const p = MOCK_CLI.find(m => m.name === args!.name);
      const config = args!.config as ProviderConfig;
      if (p) {
        MOCK_PROVIDERS[p.slug] = config;
        p.auth_mode = config.auth_mode;
        p.provider_model = config.auth_mode === "api_key" ? config.model : null;
        if (config.auth_mode === "api_key" && config.api_key) p.logged_in = true;
      }
      return undefined as T;
    }
    case "list_terminals": return structuredClone(MOCK_TERMINALS) as T;
    case "get_default_terminal": return MOCK_DEFAULT_TERMINAL as T;
    case "set_default_terminal": MOCK_DEFAULT_TERMINAL = (args!.id as string | null) ?? null; return undefined as T;
    case "get_custom_terminal_path": return MOCK_CUSTOM_TERMINAL_PATH as T;
    case "set_custom_terminal":
      MOCK_CUSTOM_TERMINAL_PATH = args!.path as string;
      MOCK_DEFAULT_TERMINAL = "custom";
      return undefined as T;
    case "pick_terminal_exe": return "/Applications/WezTerm.app/Contents/MacOS/wezterm-gui" as T;
    case "list_browsers": return structuredClone(MOCK_BROWSERS) as T;
    case "get_browser_prefs": return structuredClone(MOCK_BROWSER_PREFS) as T;
    case "set_browser_prefs": {
      MOCK_BROWSER_PREFS = {
        browser_id: (args!.browserId as string | null) ?? null,
        custom_path: (args!.customPath as string | null) ?? null,
        reuse_profile: args!.reuseProfile as boolean,
      };
      return undefined as T;
    }
    case "pick_browser_exe": return "/Applications/Firefox.app/Contents/MacOS/firefox" as T;
    case "list_ides": return ([
      { id: "vscode", name: "Visual Studio Code", cli_path: "/usr/local/bin/code" },
      { id: "cursor", name: "Cursor", cli_path: "/Applications/Cursor.app/…/cursor" },
    ] satisfies IdeInfo[]) as T;
    case "pick_folder": return "/Users/you/Projects/demo" as T;
    case "open_in_ide": console.log("open_in_ide", args); return undefined as T;
    case "list_workspaces": return structuredClone(MOCK_WS) as T;
    case "save_workspace": {
      const a = args as { ideId: string; accountSlug: string; projectPath: string; accountName: string; ideName: string; now: number };
      const id = `${a.ideId}${a.accountSlug}${a.projectPath}`;
      if (!MOCK_WS.some(w => w.id === id)) {
        MOCK_WS.push({ id, account_slug: a.accountSlug, account_name: a.accountName,
          ide_id: a.ideId, ide_name: a.ideName, project_path: a.projectPath,
          created: a.now, last_opened: a.now });
      }
      return undefined as T;
    }
    case "delete_workspace": { const i = MOCK_WS.findIndex(w => w.id === args!.id); if (i >= 0) MOCK_WS.splice(i, 1); return undefined as T; }
    case "open_workspace": console.log("open_workspace", args!.id); return undefined as T;
    default: return undefined as T;
  }
}
