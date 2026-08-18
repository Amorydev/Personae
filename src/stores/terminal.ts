import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "../lib/tauri";
import type { TerminalApp } from "../lib/types";

export const useTerminalStore = defineStore("terminal", () => {
  const terminals = ref<TerminalApp[]>([]);
  const defaultTerminalId = ref<string | null>(null);
  const customPath = ref<string | null>(null);

  async function load() {
    terminals.value = await invoke<TerminalApp[]>("list_terminals");
    defaultTerminalId.value = await invoke<string | null>("get_default_terminal");
    customPath.value = await invoke<string | null>("get_custom_terminal_path");
  }

  async function choose(id: string) {
    await invoke("set_default_terminal", { id });
    defaultTerminalId.value = id;
  }

  async function clear() {
    await invoke("set_default_terminal", { id: null });
    defaultTerminalId.value = null;
  }

  // Opens a native file picker; returns the chosen path, or null if canceled.
  async function pickCustom(): Promise<string | null> {
    const path = await invoke<string | null>("pick_terminal_exe");
    if (path) customPath.value = path;
    return path;
  }

  async function chooseCustom(path: string) {
    await invoke("set_custom_terminal", { path });
    defaultTerminalId.value = "custom";
    customPath.value = path;
  }

  return { terminals, defaultTerminalId, customPath, load, choose, clear, pickCustom, chooseCustom };
});
