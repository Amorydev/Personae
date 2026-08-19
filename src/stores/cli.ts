import { defineStore } from "pinia";
import { computed, reactive, ref, watch } from "vue";
import { invoke, nowSecs } from "../lib/tauri";
import type { CliProfile, ProviderConfig, Workspace } from "../lib/types";
import { useUiStore } from "./ui";
import { useTerminalStore } from "./terminal";

// Persisted (survives app restarts) so a transient fetch miss — or an
// account whose local .claude.json cache doesn't have fresh data on this
// particular read — falls back to the last real number we've seen instead of
// the generic plan-tier label, and so the 1h throttle below holds across a
// restart too.
const LAST_USAGE_KEY = "personae:lastUsage";
const LAST_USAGE_FETCH_KEY = "personae:lastUsageFetchAt";
const USAGE_REFRESH_THROTTLE_MS = 60 * 60 * 1000;

type UsageCache = Record<string, { session: number | null; weekly: number | null }>;

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}
function saveJson(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // best-effort cache — ignore quota/availability errors
  }
}

export const useCliStore = defineStore("cli", () => {
  const cliProfiles = ref<CliProfile[]>([]);
  const cliSelected = ref<string | null>(null); // slug
  const cliQuery = ref("");
  const cliAvailable = ref(true);
  const cliWorkspaces = ref<Workspace[]>([]); // all saved workspaces (rendered nested under their account)
  const pendingLaunch = ref<CliProfile | null>(null); // set while the first-run terminal picker is up
  const initialized = ref(false); // false until the first reloadCli() settles — gates the loading UI

  const filteredCli = computed(() => {
    const q = cliQuery.value.trim().toLowerCase();
    if (!q) return cliProfiles.value;
    return cliProfiles.value.filter(
      (p) => p.name.toLowerCase().includes(q) || String(p.account_email || "").toLowerCase().includes(q)
    );
  });
  const cliCurrent = computed(() => cliProfiles.value.find((p) => p.slug === cliSelected.value) || null);
  const workspacesFor = computed(() => (slug: string) => cliWorkspaces.value.filter((w) => w.account_slug === slug));

  watch(filteredCli, (items) => {
    if (items.length && !items.some((p) => p.slug === cliSelected.value)) cliSelected.value = items[0].slug;
  });

  // Covers every way an account becomes "current" — first account
  // auto-selected on initial load, clicking a different account, or the
  // filtered-list fallback re-select above — with one live usage refresh,
  // rather than needing a call at every individual call site that sets
  // cliSelected. Throttled to once per hour per account (see
  // refreshUsageLive) so re-selecting the same account repeatedly doesn't
  // keep hitting Anthropic's endpoint.
  //
  // Watches cliSelected (the plain slug ref), NOT cliCurrent: cliCurrent is a
  // computed `.find()` into cliProfiles, and reloadCli() replaces that whole
  // array (fresh object instances) on every call — e.g. every throttled
  // window-focus reload — so cliCurrent's *reference* changes even when the
  // selected account hasn't. Watching the primitive slug means Vue's own
  // same-value skip applies, so re-resolving to the same account after a
  // background reload correctly does NOT refire this.
  watch(cliSelected, (slug) => {
    const p = cliProfiles.value.find((x) => x.slug === slug);
    if (p) refreshUsageLive(p, { silent: true });
  });

  async function reloadCli() {
    try {
      cliAvailable.value = await invoke<boolean>("cli_available");
      cliProfiles.value = await invoke<CliProfile[]>("list_cli_profiles");
      // Backend list() reads only the CLI's own local cache, which can be
      // null for reasons unrelated to "no usage data exists" (e.g. right
      // after the account-dir migration, or before the first live fetch of
      // this session lands) — fall back to our own persisted last-known
      // value so the detail pane doesn't flash the generic plan-tier label.
      const lastUsage = loadJson<UsageCache>(LAST_USAGE_KEY, {});
      for (const p of cliProfiles.value) {
        if (p.session_usage_pct == null && p.weekly_usage_pct == null && lastUsage[p.slug]) {
          p.session_usage_pct = lastUsage[p.slug].session;
          p.weekly_usage_pct = lastUsage[p.slug].weekly;
        }
      }
      cliWorkspaces.value = await invoke<Workspace[]>("list_workspaces");
      if (!cliProfiles.value.some((p) => p.slug === cliSelected.value)) {
        cliSelected.value = cliProfiles.value[0]?.slug ?? null;
      }
    } catch (e) {
      console.error(e);
    } finally {
      initialized.value = true;
    }
  }

  const usageRefreshingSlug = ref<string | null>(null); // drives the manual refresh button's spinner
  // Accounts whose live usage fetch came back 401 (expired/invalid OAuth
  // token) — CliDetail treats this like a logged-out state (forces the
  // Relogin prompt) regardless of what the locally-computed token expiry
  // says, since the server just told us directly. Cleared the moment a live
  // fetch for that account succeeds again.
  const sessionExpiredSlugs = reactive(new Set<string>());

  function selectCliProfile(p: CliProfile) {
    cliSelected.value = p.slug;
  }

  // A REAL network call straight to Anthropic's own usage endpoint (the
  // exact lightweight, non-billed GET `claude` itself makes — see
  // cli::fetch_live_usage), not just a re-read of what the CLI last cached.
  // `silent` (used by the cliCurrent watcher — first account opened, or
  // switching accounts) swallows generic failures quietly (an expired-token
  // AUTH_EXPIRED result still surfaces — see below) since that's a passive
  // background refresh, is throttled to once per hour per account, and skips
  // an account already flagged as session-expired (nothing would succeed
  // until they relogin anyway). The manual refresh button passes
  // silent: false, which shows a toast on any failure and always bypasses
  // the throttle, since it's an explicit "get me a fresh number now" ask.
  async function refreshUsageLive(p: CliProfile, opts: { silent: boolean } = { silent: false }) {
    const ui = useUiStore();
    if (opts.silent) {
      if (sessionExpiredSlugs.has(p.slug)) return;
      const lastFetch = loadJson<Record<string, number>>(LAST_USAGE_FETCH_KEY, {});
      if (Date.now() - (lastFetch[p.slug] ?? 0) < USAGE_REFRESH_THROTTLE_MS) return;
    }
    usageRefreshingSlug.value = p.slug;
    try {
      const [sessionPct, weeklyPct] = await invoke<[number | null, number | null]>("fetch_live_cli_usage", { name: p.name });
      sessionExpiredSlugs.delete(p.slug);
      patchUsage(p.slug, sessionPct, weeklyPct);
      const lastFetch = loadJson<Record<string, number>>(LAST_USAGE_FETCH_KEY, {});
      lastFetch[p.slug] = Date.now();
      saveJson(LAST_USAGE_FETCH_KEY, lastFetch);
    } catch (e) {
      if (e === "AUTH_EXPIRED") {
        sessionExpiredSlugs.add(p.slug);
        ui.showToast(`Session expired for "${p.name}" — please sign in again.`, "error");
      } else if (opts.silent) {
        console.error(e);
      } else {
        ui.showToast(String(e), "error");
      }
    } finally {
      if (usageRefreshingSlug.value === p.slug) usageRefreshingSlug.value = null;
    }
  }

  function patchUsage(slug: string, sessionPct: number | null, weeklyPct: number | null) {
    const target = cliProfiles.value.find((x) => x.slug === slug);
    if (!target) return;
    const cache = loadJson<UsageCache>(LAST_USAGE_KEY, {});
    if (sessionPct != null || weeklyPct != null) {
      cache[slug] = { session: sessionPct, weekly: weeklyPct };
      saveJson(LAST_USAGE_KEY, cache);
      target.session_usage_pct = sessionPct;
      target.weekly_usage_pct = weeklyPct;
    } else {
      // Nothing new this time — keep showing the last real number we saw,
      // rather than blanking out to the generic plan-tier label.
      const last = cache[slug];
      target.session_usage_pct = last?.session ?? null;
      target.weekly_usage_pct = last?.weekly ?? null;
    }
  }

  function moveCliSelection(delta: number) {
    const items = filteredCli.value;
    if (!items.length) return;
    let i = items.findIndex((p) => p.slug === cliSelected.value);
    i = Math.max(0, Math.min(items.length - 1, (i < 0 ? 0 : i) + delta));
    cliSelected.value = items[i].slug;
  }

  async function doCliLogin() {
    const ui = useUiStore();
    const p = cliCurrent.value;
    if (!p) return;
    try {
      await invoke("login_cli_profile", { name: p.name });
    } catch (e) {
      ui.showToast(String(e), "error");
      return;
    }
    ui.showToast(`Sign-in opened in Terminal for "${p.name}". Return here when it is complete.`);
  }

  async function doCliLaunch(p: CliProfile) {
    const ui = useUiStore();
    const terminal = useTerminalStore();
    // First launch, more than one terminal available, no saved preference yet:
    // ask once, then resume this launch once a choice is made.
    if (terminal.terminals.length > 1 && !terminal.defaultTerminalId) {
      pendingLaunch.value = p;
      ui.openModal("terminal");
      return;
    }
    // Terminal is settled — ask which folder to launch from (remembers the
    // last 5 per account; the picker itself defaults to the most recent).
    pendingLaunch.value = p;
    ui.openModal("launchLocation");
  }

  async function doCliLaunchAt(p: CliProfile, projectPath: string | null) {
    const ui = useUiStore();
    try {
      await invoke("launch_cli_profile", { name: p.name, projectPath });
    } catch (e) {
      ui.showToast(String(e), "error");
    }
  }

  async function getLaunchHistory(name: string): Promise<string[]> {
    return invoke<string[]>("get_launch_history", { name });
  }

  async function resumePendingLaunch() {
    const p = pendingLaunch.value;
    pendingLaunch.value = null;
    if (p) await doCliLaunch(p);
  }

  // `provider` is only passed for the "sign in with an API key" creation path;
  // omit it (or pass an "oauth" mode config) to keep the original claude.ai
  // sign-in-in-Terminal flow.
  async function doCliCreate(name: string, provider?: ProviderConfig) {
    await invoke("create_cli_profile", { name });
    if (provider && provider.auth_mode === "api_key") {
      await invoke("set_cli_provider_config", { name, config: provider });
    }
    await reloadCli();
    const made = cliProfiles.value.find((p) => p.name === name);
    if (made) {
      cliSelected.value = made.slug;
      if (!provider || provider.auth_mode !== "api_key") await doCliLogin();
    }
  }

  async function doCliDelete(name: string) {
    await invoke("delete_cli_profile", { name, purge: true });
    await reloadCli();
  }

  // Re-selects by the NEW name after reload rather than assuming the slug is
  // unchanged: on Windows it always is (slug stays tied to the original
  // name), but a macOS rename that changes the slugified form moves the
  // account under a new slug — see cli::rename's doc comment.
  async function renameCliProfile(oldName: string, newName: string) {
    await invoke("rename_cli_profile", { oldName, newName });
    await reloadCli();
    const renamed = cliProfiles.value.find((p) => p.name === newName);
    if (renamed) cliSelected.value = renamed.slug;
  }

  async function getProviderConfig(name: string): Promise<ProviderConfig> {
    return invoke<ProviderConfig>("get_cli_provider_config", { name });
  }

  async function setProviderConfig(name: string, config: ProviderConfig) {
    await invoke("set_cli_provider_config", { name, config });
    await reloadCli();
  }

  async function deleteWorkspace(id: string) {
    const ui = useUiStore();
    try {
      await invoke("delete_workspace", { id });
    } catch (e) {
      ui.showToast(String(e), "error");
    }
    await reloadCli();
  }

  async function openWorkspace(id: string) {
    const ui = useUiStore();
    try {
      await invoke("open_workspace", { id, now: nowSecs() });
    } catch (e) {
      ui.showToast(String(e), "error");
    }
    await reloadCli();
  }

  async function openInIde({ ideId, ideName, projectPath }: { ideId: string; ideName: string; projectPath: string }) {
    const p = cliCurrent.value;
    if (!p) return;
    await invoke("open_in_ide", { account: p.name, ideId, projectPath });
    await invoke("save_workspace", {
      accountSlug: p.slug,
      accountName: p.name,
      ideId,
      ideName,
      projectPath,
      now: nowSecs(),
    });
    await reloadCli();
  }

  return {
    cliProfiles, cliSelected, cliQuery, cliAvailable, cliWorkspaces, pendingLaunch, initialized, usageRefreshingSlug, sessionExpiredSlugs,
    filteredCli, cliCurrent, workspacesFor,
    reloadCli, selectCliProfile, moveCliSelection, doCliLogin, doCliLaunch, doCliLaunchAt, getLaunchHistory, resumePendingLaunch, doCliCreate, doCliDelete, renameCliProfile,
    deleteWorkspace, openWorkspace, openInIde, getProviderConfig, setProviderConfig, refreshUsageLive,
  };
});
