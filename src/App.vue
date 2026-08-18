<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useUiStore } from "./stores/ui";
import { useDesktopStore } from "./stores/desktop";
import { useCliStore } from "./stores/cli";
import { useTerminalStore } from "./stores/terminal";
import { useGlobalShortcuts } from "./composables/useGlobalShortcuts";
import { cliColor } from "./lib/format";
import DesktopSidebar from "./components/desktop/DesktopSidebar.vue";
import DesktopDetail from "./components/desktop/DesktopDetail.vue";
import CreateProfileModal from "./components/desktop/CreateProfileModal.vue";
import EditAccentModal from "./components/desktop/EditAccentModal.vue";
import DeleteProfileModal from "./components/desktop/DeleteProfileModal.vue";
import CliSidebar from "./components/cli/CliSidebar.vue";
import CliDetail from "./components/cli/CliDetail.vue";
import CliCreateModal from "./components/cli/CliCreateModal.vue";
import CliDeleteModal from "./components/cli/CliDeleteModal.vue";
import IdeModal from "./components/cli/IdeModal.vue";
import ProviderConfigModal from "./components/cli/ProviderConfigModal.vue";
import TerminalPickerModal from "./components/cli/TerminalPickerModal.vue";
import SettingsModal from "./components/cli/SettingsModal.vue";
import LaunchLocationModal from "./components/cli/LaunchLocationModal.vue";
import Toast from "./components/Toast.vue";
import { useThemeStore } from "./stores/theme";

const ui = useUiStore();
const desktop = useDesktopStore();
const cli = useCliStore();
const terminal = useTerminalStore();
useThemeStore(); // applies the saved/system theme on mount; no local state needed here

useGlobalShortcuts();

const desktopAccent = computed(() => desktop.current?.tint || "#c2714f");
const cliAccent = computed(() => (cli.cliCurrent ? cliColor(cli.cliCurrent) : "#c2714f"));

function setView(v: "desktop" | "cli") {
  ui.setView(v);
  if (v === "cli") {
    cli.reloadCli();
    terminal.load();
  }
}

onMounted(() => {
  desktop.reload();
  window.addEventListener("focus", () => {
    if (ui.view === "cli") cli.reloadCli();
  });
});
</script>

<template>
  <div id="app-desktop" class="app" :class="{ hidden: ui.view === 'cli' }" :style="{ '--accent': desktopAccent }">
    <DesktopSidebar />
    <main class="detail">
      <div class="detail-head" data-tauri-drag-region>
        <div class="spacer" data-tauri-drag-region></div>
        <div class="segmented">
          <button class="seg" :class="{ sel: ui.view === 'desktop' }" @click="setView('desktop')">Desktop</button>
          <button class="seg" :class="{ sel: ui.view === 'cli' }" @click="setView('cli')">CLI</button>
        </div>
        <div class="spacer" data-tauri-drag-region></div>
      </div>
      <div v-if="!desktop.claudeFound" class="warn">⚠️ Claude Desktop not found. Install it, or set CLAUDE_APP.</div>
      <div class="detail-body desktop-detail-body">
        <DesktopDetail />
      </div>
    </main>
  </div>

  <div id="cli-view" class="app" :class="{ hidden: ui.view !== 'cli' }" :style="{ '--accent': cliAccent }">
    <CliSidebar />
    <main class="detail cli-detail">
      <div class="detail-head" data-tauri-drag-region>
        <div class="spacer" data-tauri-drag-region></div>
        <div class="segmented">
          <button class="seg" :class="{ sel: ui.view === 'desktop' }" @click="setView('desktop')">Desktop</button>
          <button class="seg" :class="{ sel: ui.view === 'cli' }" @click="setView('cli')">CLI</button>
        </div>
        <div class="spacer" data-tauri-drag-region></div>
      </div>
      <div class="detail-body cli-detail-body">
        <CliDetail />
      </div>
    </main>
  </div>

  <CreateProfileModal />
  <EditAccentModal />
  <DeleteProfileModal />
  <CliCreateModal />
  <CliDeleteModal />
  <IdeModal />
  <ProviderConfigModal />
  <TerminalPickerModal />
  <SettingsModal />
  <LaunchLocationModal />
  <Toast />
</template>
