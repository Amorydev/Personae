<script setup lang="ts">
import { ref, watch } from "vue";
import { useDesktopSettingsStore } from "../../stores/desktopSettings";
import { useUiStore } from "../../stores/ui";

const desktopSettings = useDesktopSettingsStore();
const ui = useUiStore();

const exePath = ref("");

watch(
  () => ui.modals.desktopSettings,
  async (open) => {
    if (!open) return;
    if (!desktopSettings.loaded) await desktopSettings.load();
    exePath.value = desktopSettings.exeOverride ?? "";
  }
);

async function pick() {
  const path = await desktopSettings.pickCustom();
  if (path) exePath.value = path;
}

async function save() {
  await desktopSettings.save(exePath.value.trim() || null);
  ui.closeModal("desktopSettings");
}
</script>

<template>
  <div v-if="ui.modals.desktopSettings" class="modal" role="dialog" aria-modal="true" aria-labelledby="desktop-settings-title" @click.self="ui.closeModal('desktopSettings')">
    <div class="modal-card settings-card">
      <div id="desktop-settings-title" class="modal-title">Settings</div>

      <div class="settings-section">
        <div class="section-heading"><div><h2>Claude Desktop executable</h2><p>What "Launch" opens. Leave blank to auto-detect the installed copy.</p></div></div>
        <label class="field-group">
          <span class="k">Path to claude.exe</span>
          <div class="key-row">
            <input v-model="exePath" class="text" type="text" placeholder="Auto-detect" spellcheck="false" />
            <button class="btn compact" @click="pick">Browse…</button>
          </div>
        </label>
      </div>

      <div class="modal-actions">
        <button class="btn" @click="ui.closeModal('desktopSettings')">Cancel</button>
        <button class="primary" @click="save">Save</button>
      </div>
    </div>
  </div>
</template>
