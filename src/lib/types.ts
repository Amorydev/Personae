export interface DesktopProfile {
  name: string;
  slug: string;
  tint: string;
  running: boolean;
  data_size: string;
  data_path: string;
  created: number;
  last_active: number | null;
}

export type AuthMode = "oauth" | "api_key";

export interface CliProfile {
  name: string;
  slug: string;
  config_dir: string;
  launcher_path: string;
  logged_in: boolean;
  account_email: string | null;
  data_size: string;
  created: number;
  last_active: number | null;
  auth_mode: AuthMode;
  provider_model: string | null;
  token_expires_at: number | null; // unix ms — short-lived access token, auto-refreshed, not user-actionable
  refresh_expires_at: number | null; // unix ms — when this actually requires a real re-login
  subscription_type: string | null; // e.g. "max", "pro" — plan tier, not live usage
  rate_limit_tier: string | null; // e.g. "default_claude_max_20x"
  session_usage_pct: number | null; // 0-100, rolling 5-hour window; null until the account's made a real request
  weekly_usage_pct: number | null; // 0-100, plan's overall weekly cap
}

export interface ProviderConfig {
  auth_mode: AuthMode;
  base_url: string | null;
  api_key: string | null;
  model: string | null;
  small_fast_model: string | null;
}

export interface TerminalApp {
  id: string;
  name: string;
}

export interface BrowserApp {
  id: string;
  name: string;
}

export interface BrowserPrefs {
  browser_id: string | null;
  custom_path: string | null;
  reuse_profile: boolean;
}

export interface Workspace {
  id: string;
  account_slug: string;
  account_name: string;
  ide_id: string;
  ide_name: string;
  project_path: string;
  created: number;
  last_opened: number;
}

export interface IdeInfo {
  id: string;
  name: string;
  cli_path: string;
}
