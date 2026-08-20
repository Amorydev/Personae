import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { invoke } from "../lib/tauri";
import type { DesktopProfile } from "../lib/types";
import { useUiStore } from "./ui";

export const useDesktopStore = defineStore("desktop", () => {
  const profiles = ref<DesktopProfile[]>([]);
  const selected = ref<string | null>(null); // slug
  const query = ref("");
  const claudeFound = ref(true);
  const initialized = ref(false); // false until the first reload() settles — gates the loading UI
  const usageRefreshingSlug = ref<string | null>(null); // drives the manual refresh button's spinner

  const filtered = computed(() => {
    const q = query.value.trim().toLowerCase();
    return q ? profiles.value.filter((p) => p.name.toLowerCase().includes(q)) : profiles.value;
  });
  const current = computed(() => profiles.value.find((p) => p.slug === selected.value) || null);

  // Mirrors the vanilla renderList() side effect: if the current selection scrolls out of the
  // filtered/search view, fall back to the first visible item.
  watch(filtered, (items) => {
    if (!items.some((p) => p.slug === selected.value)) selected.value = items[0]?.slug ?? null;
  });

  async function reload(keep = true) {
    try {
      claudeFound.value = await invoke<boolean>("claude_found");
      profiles.value = await invoke<DesktopProfile[]>("list_profiles");
      if (!keep || !profiles.value.some((p) => p.slug === selected.value)) {
        selected.value = profiles.value[0]?.slug ?? null;
      }
    } catch (e) {
      console.error(e);
    } finally {
      initialized.value = true;
    }
  }

  function select(slug: string) {
    selected.value = slug;
  }

  function moveSelection(delta: number) {
    const items = filtered.value;
    if (!items.length) return;
    let i = items.findIndex((p) => p.slug === selected.value);
    i = Math.max(0, Math.min(items.length - 1, (i < 0 ? 0 : i) + delta));
    selected.value = items[i].slug;
  }

  // Re-read the profile's newest usage sample (the desktop app keeps
  // plan-usage-history.json current while it runs — see platform::fetch_usage).
  // The desktop analog of cli's refreshUsageLive: an explicit "get me the
  // latest number now" button. When nothing new has been sampled yet, the
  // existing value is left untouched rather than blanking to "—".
  async function refreshUsageLive(p: DesktopProfile) {
    const ui = useUiStore();
    usageRefreshingSlug.value = p.slug;
    try {
      const [sessionPct, weeklyPct] = await invoke<[number | null, number | null]>("fetch_desktop_usage", { name: p.name });
      const target = profiles.value.find((x) => x.slug === p.slug);
      if (target && (sessionPct != null || weeklyPct != null)) {
        target.session_usage_pct = sessionPct;
        target.weekly_usage_pct = weeklyPct;
      }
    } catch (e) {
      ui.showToast(String(e), "error");
    } finally {
      if (usageRefreshingSlug.value === p.slug) usageRefreshingSlug.value = null;
    }
  }

  async function launch(p: DesktopProfile) {
    const ui = useUiStore();
    try {
      await invoke("launch_profile", { name: p.name });
    } catch (e) {
      ui.showToast(String(e), "error");
      return;
    }
    setTimeout(() => reload(), 1200);
  }

  async function quit(p: DesktopProfile) {
    const ui = useUiStore();
    try {
      await invoke("quit_profile", { name: p.name });
    } catch (e) {
      ui.showToast(String(e), "error");
      return;
    }
    setTimeout(() => reload(), 600);
  }

  async function create(name: string, color: string) {
    await invoke("create_profile", { name, color, isolation: "env" });
    await reload(false);
    const made = profiles.value.find((p) => p.name === name);
    if (made) selected.value = made.slug;
  }

  async function setColor(name: string, hex: string) {
    await invoke("set_profile_color", { name, color: hex });
    await reload();
  }

  async function repair() {
    await invoke("repair_profiles");
    await reload();
  }

  async function remove(name: string, purge: boolean) {
    await invoke("delete_profile", { name, purge });
    await reload(false);
  }

  return {
    profiles, selected, query, claudeFound, initialized, usageRefreshingSlug,
    filtered, current,
    reload, select, moveSelection, refreshUsageLive, launch, quit, create, setColor, repair, remove,
  };
});
