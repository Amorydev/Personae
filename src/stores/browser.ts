import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "../lib/tauri";
import type { BrowserApp } from "../lib/types";

export const useBrowserStore = defineStore("browser", () => {
  const browsers = ref<BrowserApp[]>([]);
  const browserId = ref<string | null>(null); // null = system default
  const customPath = ref<string | null>(null);
  const reuseProfile = ref(true);
  const loaded = ref(false);

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

  return { browsers, browserId, customPath, reuseProfile, loaded, load, save, pickCustom };
});
