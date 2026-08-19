<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";

const cli = useCliStore();
const ui = useUiStore();
const name = ref("");
const nameInput = ref<HTMLInputElement | null>(null);

watch(
  () => ui.modals.cliEdit,
  async (open) => {
    if (!open) return;
    name.value = cli.cliCurrent?.name ?? "";
    await nextTick();
    nameInput.value?.focus();
    nameInput.value?.select();
  }
);

async function save() {
  const p = cli.cliCurrent;
  const newName = name.value.trim();
  if (!p || !newName || newName === p.name) {
    ui.closeModal("cliEdit");
    return;
  }
  try {
    await cli.renameCliProfile(p.name, newName);
    ui.closeModal("cliEdit");
  } catch (e) {
    ui.showToast(String(e), "error");
  }
}
</script>

<template>
  <div v-if="ui.modals.cliEdit" class="modal" role="dialog" aria-modal="true" aria-labelledby="cli-edit-title" @click.self="ui.closeModal('cliEdit')">
    <div class="modal-card">
      <div id="cli-edit-title" class="modal-title">Edit account</div>
      <label class="k" for="cli-edit-name">Name</label>
      <input
        id="cli-edit-name"
        ref="nameInput"
        v-model="name"
        class="text"
        placeholder="e.g. Work, Personal, Client"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter="save"
      />
      <div class="modal-actions">
        <button class="btn" @click="ui.closeModal('cliEdit')">Cancel</button>
        <button class="primary" @click="save">Save</button>
      </div>
    </div>
  </div>
</template>
