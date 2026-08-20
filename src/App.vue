<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useUiStore } from "./stores/ui";
import { useDesktopStore } from "./stores/desktop";
import { useCliStore } from "./stores/cli";
import { useGlobalShortcuts } from "./composables/useGlobalShortcuts";
import { accentInk, cliColor } from "./lib/format";
import DesktopSidebar from "./components/desktop/DesktopSidebar.vue";
import DesktopDetail from "./components/desktop/DesktopDetail.vue";
import CreateProfileModal from "./components/desktop/CreateProfileModal.vue";
import EditAccentModal from "./components/desktop/EditAccentModal.vue";
import DeleteProfileModal from "./components/desktop/DeleteProfileModal.vue";
import DesktopSettingsModal from "./components/desktop/DesktopSettingsModal.vue";
import CliSidebar from "./components/cli/CliSidebar.vue";
import CliDetail from "./components/cli/CliDetail.vue";
import CliCreateModal from "./components/cli/CliCreateModal.vue";
import CliEditModal from "./components/cli/CliEditModal.vue";
import CliDeleteModal from "./components/cli/CliDeleteModal.vue";
import IdeModal from "./components/cli/IdeModal.vue";
import ProviderConfigModal from "./components/cli/ProviderConfigModal.vue";
import TerminalPickerModal from "./components/cli/TerminalPickerModal.vue";
import BrowserProfilePickerModal from "./components/cli/BrowserProfilePickerModal.vue";
import SettingsModal from "./components/cli/SettingsModal.vue";
import LaunchLocationModal from "./components/cli/LaunchLocationModal.vue";
import Toast from "./components/Toast.vue";
import ErrorModal from "./components/ErrorModal.vue";
import { useThemeStore } from "./stores/theme";

const ui = useUiStore();
const desktop = useDesktopStore();
const cli = useCliStore();
useThemeStore(); // applies the saved/system theme on mount; no local state needed here

useGlobalShortcuts();

const desktopAccent = computed(() => desktop.current?.tint || "#c2714f");
const cliAccent = computed(() => (cli.cliCurrent ? cliColor(cli.cliCurrent) : "#c2714f"));
const desktopInk = computed(() => accentInk(desktopAccent.value));
const cliInk = computed(() => accentInk(cliAccent.value));

onMounted(() => {
  desktop.reload();
  // This is a desktop app, not a webpage — the WebView's default
  // browser-style right-click menu ("Back", "Reload", "Inspect Element", …)
  // has no place here.
  window.addEventListener("contextmenu", (e) => e.preventDefault());
  // Throttled (leading-edge): fires on focus (e.g. returning from the Terminal
  // after `claude auth login`, to re-detect it), but coalesces rapid focus/blur
  // cycles so the CLI probes don't re-run more than once per ~800ms.
  let lastCliFocusReload = 0;
  window.addEventListener("focus", () => {
    if (ui.view !== "cli") return;
    const t = Date.now();
    if (t - lastCliFocusReload < 800) return;
    lastCliFocusReload = t;
    cli.reloadCli();
  });
});
</script>

<template>
  <div id="app-desktop" class="app" :class="{ hidden: ui.view === 'cli' }" :style="{ '--accent': desktopAccent, '--accent-ink': desktopInk }">
    <DesktopSidebar />
    <main class="detail">
      <div class="detail-head" data-tauri-drag-region></div>
      <div v-if="!desktop.claudeFound" class="warn">⚠️ Claude Desktop not found. Install it, or set its location in ⚙ Settings.</div>
      <div class="detail-body desktop-detail-body">
        <DesktopDetail />
      </div>
    </main>
  </div>

  <div id="cli-view" class="app" :class="{ hidden: ui.view !== 'cli' }" :style="{ '--accent': cliAccent, '--accent-ink': cliInk }">
    <CliSidebar />
    <main class="detail cli-detail">
      <div class="detail-head" data-tauri-drag-region></div>
      <div class="detail-body cli-detail-body">
        <CliDetail />
      </div>
    </main>
  </div>

  <CreateProfileModal />
  <EditAccentModal />
  <DeleteProfileModal />
  <DesktopSettingsModal />
  <CliCreateModal />
  <CliEditModal />
  <CliDeleteModal />
  <IdeModal />
  <ProviderConfigModal />
  <TerminalPickerModal />
  <BrowserProfilePickerModal />
  <SettingsModal />
  <LaunchLocationModal />
  <ErrorModal />
  <Toast />
</template>
