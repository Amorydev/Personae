<script setup lang="ts">
import { ref, watch } from "vue";
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";
import { invoke } from "../../lib/tauri";
import type { IdeInfo } from "../../lib/types";

const cli = useCliStore();
const ui = useUiStore();

const ides = ref<IdeInfo[]>([]);
const selectedIdeId = ref("");
const folder = ref<string | null>(null);
const status = ref("");

watch(
  () => ui.modals.ide,
  async (open) => {
    if (!open) return;
    const p = cli.cliCurrent;
    if (!p) { ui.closeModal("ide"); return; }
    if (!p.logged_in) {
      ui.closeModal("ide");
      ui.showToast("Log in to this account first.", "error");
      return;
    }
    const list = await invoke<IdeInfo[]>("list_ides");
    if (!list.length) {
      ui.closeModal("ide");
      ui.showToast("No VS Code-family IDE found (VS Code, Cursor, Windsurf, Antigravity).", "error");
      return;
    }
    ides.value = list;
    selectedIdeId.value = list[0].id;
    folder.value = null;
    status.value = "";
  }
);

async function pickFolder() {
  const path = await invoke<string | null>("pick_folder");
  if (path) folder.value = path;
}

async function open() {
  if (!folder.value) {
    status.value = "Choose a project folder first.";
    return;
  }
  const ide = ides.value.find((i) => i.id === selectedIdeId.value);
  if (!ide) return;
  status.value = "Opening…";
  try {
    await cli.openInIde({ ideId: ide.id, ideName: ide.name, projectPath: folder.value });
  } catch (e) {
    status.value = String(e);
    return;
  }
  ui.closeModal("ide");
}
</script>

<template>
  <div v-if="ui.modals.ide" class="modal" role="dialog" aria-modal="true" aria-labelledby="ide-modal-title" @click.self="ui.closeModal('ide')">
    <div class="modal-card ide-modal-card">
      <div id="ide-modal-title" class="modal-title">Open a project</div>
      <p class="modal-lead">Open a project with <strong>{{ cli.cliCurrent?.name }}</strong> already active in its terminals.</p>
      <label class="k" for="ide-select">IDE</label>
      <div class="select-wrap">
        <select id="ide-select" v-model="selectedIdeId" class="text">
          <option v-for="i in ides" :key="i.id" :value="i.id">{{ i.name }}</option>
        </select>
      </div>
      <label class="k">Project folder</label>
      <button class="folder-picker" @click="pickFolder">
        <span>{{ folder || "No folder chosen" }}</span>
        <span class="folder-picker-action">Choose…</span>
      </button>
      <p class="info-note"><span aria-hidden="true">↳</span><span>Personae configures <code>CLAUDE_CONFIG_DIR</code> for this project's terminals and opens a Claude terminal automatically.</span></p>
      <div class="hint">{{ status }}</div>
      <div class="modal-actions">
        <button class="btn" @click="ui.closeModal('ide')">Cancel</button>
        <button class="primary" @click="open">Open</button>
      </div>
    </div>
  </div>
</template>
