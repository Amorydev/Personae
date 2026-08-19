import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type ThemeMode = "light" | "dark" | "system";

const STORAGE_KEY = "personae-theme";

function apply(mode: ThemeMode) {
  const root = document.documentElement;
  if (mode === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", mode);
}

export const useThemeStore = defineStore("theme", () => {
  const stored = (localStorage.getItem(STORAGE_KEY) as ThemeMode | null) ?? "system";
  const mode = ref<ThemeMode>(stored === "light" || stored === "dark" || stored === "system" ? stored : "system");
  apply(mode.value);

  watch(mode, (m) => {
    localStorage.setItem(STORAGE_KEY, m);
    apply(m);
  });

  function setMode(m: ThemeMode) {
    mode.value = m;
  }

  return { mode, setMode };
});
