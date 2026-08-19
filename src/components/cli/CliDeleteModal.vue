<script setup lang="ts">
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";

const cli = useCliStore();
const ui = useUiStore();

async function confirmDelete() {
  const p = cli.cliCurrent;
  if (!p) return;
  ui.closeModal("cliDel");
  try {
    await cli.doCliDelete(p.name);
  } catch (e) {
    ui.showToast(String(e), "error");
  }
}
</script>

<template>
  <div v-if="ui.modals.cliDel" class="modal" role="dialog" aria-modal="true" aria-labelledby="cli-del-title" @click.self="ui.closeModal('cliDel')">
    <div class="modal-card">
      <div id="cli-del-title" class="modal-title">Delete "{{ cli.cliCurrent?.name }}"?</div>
      <p class="hint">This removes the launcher, its login, and its CLI config directory. This cannot be undone.</p>
      <div class="modal-actions">
        <button class="btn" @click="ui.closeModal('cliDel')">Cancel</button>
        <button class="danger" @click="confirmDelete">Delete account</button>
      </div>
    </div>
  </div>
</template>
