<script setup lang="ts">
import { ref, watch } from "vue";
import { useTerminalStore } from "../../stores/terminal";
import { useBrowserStore } from "../../stores/browser";
import { useUiStore } from "../../stores/ui";

const terminal = useTerminalStore();
const browser = useBrowserStore();
const ui = useUiStore();

const selectedTerminal = ref("");
const selectedBrowser = ref("system"); // "system" | "chrome" | "edge" | "custom"
const browserCustomPath = ref<string | null>(null);
const reuseProfile = ref(true);

watch(
  () => ui.modals.settings,
  async (open) => {
    if (!open) return;
    if (!terminal.terminals.length) await terminal.load();
    if (!browser.loaded) await browser.load();
    selectedTerminal.value = terminal.defaultTerminalId || terminal.terminals[0]?.id || "";
    selectedBrowser.value = browser.browserId ?? "system";
    browserCustomPath.value = browser.customPath;
    reuseProfile.value = browser.reuseProfile;
  }
);

function terminalCustomLabel(): string {
  if (!terminal.customPath) return "Custom program…";
  const name = terminal.customPath.split(/[\\/]/).pop() || terminal.customPath;
  return `Custom: ${name}`;
}
async function pickCustomTerminal() {
  const path = await terminal.pickCustom();
  if (path) selectedTerminal.value = "custom";
}

function browserCustomLabel(): string {
  if (!browserCustomPath.value) return "Custom program…";
  const name = browserCustomPath.value.split(/[\\/]/).pop() || browserCustomPath.value;
  return `Custom: ${name}`;
}
async function pickCustomBrowser() {
  const path = await browser.pickCustom();
  if (path) {
    browserCustomPath.value = path;
    selectedBrowser.value = "custom";
  }
}

async function save() {
  if (selectedTerminal.value === "custom") {
    if (terminal.customPath) await terminal.chooseCustom(terminal.customPath);
  } else if (selectedTerminal.value) {
    await terminal.choose(selectedTerminal.value);
  }

  const id = selectedBrowser.value === "system" ? null : selectedBrowser.value;
  const path = selectedBrowser.value === "custom" ? browserCustomPath.value : null;
  await browser.save(id, path, reuseProfile.value);

  ui.closeModal("settings");
}
</script>

<template>
  <div v-if="ui.modals.settings" class="modal" role="dialog" aria-modal="true" aria-labelledby="settings-title" @click.self="ui.closeModal('settings')">
    <div class="modal-card settings-card">
      <div id="settings-title" class="modal-title">Settings</div>

      <div class="settings-section">
        <div class="section-heading"><div><h2>Terminal</h2><p>Where "Launch CLI" opens an account.</p></div></div>
        <div class="terminal-options">
          <label v-for="t in terminal.terminals" :key="t.id" class="chk">
            <input v-model="selectedTerminal" type="radio" name="settings-terminal" :value="t.id" /> {{ t.name }}
          </label>
          <label class="chk">
            <input type="radio" name="settings-terminal" value="custom" :checked="selectedTerminal === 'custom'" @click.prevent="pickCustomTerminal" />
            {{ terminalCustomLabel() }}
          </label>
        </div>
      </div>

      <div class="settings-section">
        <div class="section-heading"><div><h2>Sign-in browser</h2><p>What opens for claude.ai sign-in.</p></div></div>
        <div class="terminal-options">
          <label class="chk"><input v-model="selectedBrowser" type="radio" name="settings-browser" value="system" /> System default</label>
          <label v-for="b in browser.browsers" :key="b.id" class="chk">
            <input v-model="selectedBrowser" type="radio" name="settings-browser" :value="b.id" /> {{ b.name }}
          </label>
          <label class="chk">
            <input type="radio" name="settings-browser" value="custom" :checked="selectedBrowser === 'custom'" @click.prevent="pickCustomBrowser" />
            {{ browserCustomLabel() }}
          </label>
        </div>
        <label class="chk field-group">
          <input v-model="reuseProfile" type="checkbox" />
          Reuse my existing sign-in there (uncheck to always start a fresh, signed-out session)
        </label>
      </div>

      <div class="modal-actions">
        <button class="btn" @click="ui.closeModal('settings')">Cancel</button>
        <button class="primary" @click="save">Save</button>
      </div>
    </div>
  </div>
</template>
