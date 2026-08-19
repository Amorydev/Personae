import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { invoke, nowSecs } from "../lib/tauri";
import type { CliProfile, ProviderConfig, Workspace } from "../lib/types";
import { useUiStore } from "./ui";
import { useTerminalStore } from "./terminal";

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
    if (items.length && !items.some((p) => p.slug === cliSelected.value)) {
      cliSelected.value = items[0].slug;
      refreshUsage(items[0]);
    }
  });

  async function reloadCli() {
    try {
      cliAvailable.value = await invoke<boolean>("cli_available");
      cliProfiles.value = await invoke<CliProfile[]>("list_cli_profiles");
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

  function selectCliProfile(p: CliProfile) {
    cliSelected.value = p.slug;
    refreshUsage(p);
  }

  // Fire-and-forget: re-reads just this account's cached /usage numbers from
  // disk and patches them in reactively. Never awaited by callers — selecting
  // an account should never block on it, and a failure here shouldn't surface
  // as an error toast for what's just a background refresh.
  async function refreshUsage(p: CliProfile) {
    try {
      const [sessionPct, weeklyPct] = await invoke<[number | null, number | null]>("get_cli_usage", { name: p.name });
      const target = cliProfiles.value.find((x) => x.slug === p.slug);
      if (target) {
        target.session_usage_pct = sessionPct;
        target.weekly_usage_pct = weeklyPct;
      }
    } catch (e) {
      console.error(e);
    }
  }

  function moveCliSelection(delta: number) {
    const items = filteredCli.value;
    if (!items.length) return;
    let i = items.findIndex((p) => p.slug === cliSelected.value);
    i = Math.max(0, Math.min(items.length - 1, (i < 0 ? 0 : i) + delta));
    cliSelected.value = items[i].slug;
    refreshUsage(items[i]);
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
    cliProfiles, cliSelected, cliQuery, cliAvailable, cliWorkspaces, pendingLaunch, initialized,
    filteredCli, cliCurrent, workspacesFor,
    reloadCli, selectCliProfile, moveCliSelection, doCliLogin, doCliLaunch, doCliLaunchAt, getLaunchHistory, resumePendingLaunch, doCliCreate, doCliDelete,
    deleteWorkspace, openWorkspace, openInIde, getProviderConfig, setProviderConfig,
  };
});
