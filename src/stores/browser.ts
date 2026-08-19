import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "../lib/tauri";
import type { BrowserApp, BrowserProfile } from "../lib/types";

export const useBrowserStore = defineStore("browser", () => {
  const browsers = ref<BrowserApp[]>([]);
  const browserId = ref<string | null>(null); // null = system default
  const customPath = ref<string | null>(null);
  const reuseProfile = ref(true);
  const loaded = ref(false);
  // Chromium profiles of whichever browser sign-in will use. Empty for a
  // non-Chromium or custom browser, which callers read as "nothing to pick".
  const profiles = ref<BrowserProfile[]>([]);
  const profilesLoaded = ref(false);

  async function load() {
    browsers.value = await invoke<BrowserApp[]>("list_browsers");
    const prefs = await invoke<{ browser_id: string | null; custom_path: string | null; reuse_profile: boolean }>("get_browser_prefs");
    browserId.value = prefs.browser_id;
    customPath.value = prefs.custom_path;
    reuseProfile.value = prefs.reuse_profile;
    loaded.value = true;
  }

  async function save(id: string | null, path: string | null, reuse: boolean) {
    await invoke("set_browser_prefs", { browserId: id, customPath: path, reuseProfile: reuse });
    browserId.value = id;
    customPath.value = path;
    reuseProfile.value = reuse;
  }

  async function pickCustom(): Promise<string | null> {
    return invoke<string | null>("pick_browser_exe");
  }

  async function loadProfiles() {
    profiles.value = await invoke<BrowserProfile[]>("list_browser_profiles");
    profilesLoaded.value = true;
  }

  /** Which browser profile signs this account in, or null if never chosen. */
  async function accountProfile(slug: string): Promise<string | null> {
    return invoke<string | null>("get_account_browser_profile", { slug });
  }

  async function setAccountProfile(slug: string, profileDir: string | null) {
    await invoke("set_account_browser_profile", { slug, profileDir });
  }

  return {
    browsers, browserId, customPath, reuseProfile, loaded, profiles, profilesLoaded,
    load, save, pickCustom, loadProfiles, accountProfile, setAccountProfile,
  };
});
