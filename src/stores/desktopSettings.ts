import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "../lib/tauri";

export const useDesktopSettingsStore = defineStore("desktopSettings", () => {
  const exeOverride = ref<string | null>(null);
  const loaded = ref(false);

  async function load() {
    exeOverride.value = await invoke<string | null>("get_desktop_exe_override");
    loaded.value = true;
  }

  async function save(path: string | null) {
    await invoke("set_desktop_exe_override", { path });
    exeOverride.value = path;
  }

  async function pickCustom(): Promise<string | null> {
    return invoke<string | null>("pick_desktop_exe");
  }

  return { exeOverride, loaded, load, save, pickCustom };
});
