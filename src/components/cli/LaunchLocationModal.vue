<script setup lang="ts">
import { ref, watch } from "vue";
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";
import { invoke } from "../../lib/tauri";

const cli = useCliStore();
const ui = useUiStore();

const history = ref<string[]>([]);
const selected = ref<string | null>(null);

watch(
  () => ui.modals.launchLocation,
  async (open) => {
    if (!open) return;
    const p = cli.pendingLaunch;
    if (!p) {
      ui.closeModal("launchLocation");
      return;
    }
    history.value = await cli.getLaunchHistory(p.name);
    selected.value = history.value[0] ?? null; // "remembers the last location"
  }
);

function folderName(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || path;
}

async function browse() {
  const path = await invoke<string | null>("pick_folder");
  if (path) selected.value = path;
}

async function launch() {
  const p = cli.pendingLaunch;
  cli.pendingLaunch = null;
  ui.closeModal("launchLocation");
  if (p) await cli.doCliLaunchAt(p, selected.value);
}

function cancel() {
  ui.closeModal("launchLocation");
  cli.pendingLaunch = null;
}
</script>

<template>
  <div v-if="ui.modals.launchLocation" class="modal" role="dialog" aria-modal="true" aria-labelledby="launch-location-title" @click.self="cancel">
    <div class="modal-card">
      <div id="launch-location-title" class="modal-title">Launch from…</div>
      <p class="modal-lead">Open <strong>{{ cli.pendingLaunch?.name }}</strong>'s terminal starting in this folder.</p>

      <div v-if="history.length" class="terminal-options">
        <label v-for="path in history" :key="path" class="chk">
          <input v-model="selected" type="radio" name="launch-location" :value="path" />
          <span class="workspace-copy"><strong>{{ folderName(path) }}</strong><span>{{ path }}</span></span>
        </label>
      </div>
      <p v-else class="hint">No recent folders yet for this account.</p>

      <button class="folder-picker field-group" @click="browse">
        <span>{{ selected && !history.includes(selected) ? selected : "Browse for a folder…" }}</span>
        <span class="folder-picker-action">Choose…</span>
      </button>

      <div class="modal-actions">
        <button class="btn" @click="cancel">Cancel</button>
        <button class="primary" @click="launch">Launch</button>
      </div>
    </div>
  </div>
</template>
