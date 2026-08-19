<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { useDesktopStore } from "../../stores/desktop";
import { useUiStore } from "../../stores/ui";
import { PALETTE } from "../../lib/format";
import SwatchPicker from "../SwatchPicker.vue";

const desktop = useDesktopStore();
const ui = useUiStore();
const name = ref("");
const color = ref(PALETTE[0]);
const nameInput = ref<HTMLInputElement | null>(null);

watch(
  () => ui.modals.create,
  (open) => {
    if (!open) return;
    name.value = "";
    color.value = PALETTE[0];
    nextTick(() => nameInput.value?.focus());
  }
);

async function submit() {
  const n = name.value.trim();
  if (!n) return;
  try {
    await desktop.create(n, color.value);
  } catch (e) {
    ui.showToast(String(e), "error");
    return;
  }
  ui.closeModal("create");
}
</script>

<template>
  <div v-if="ui.modals.create" class="modal" role="dialog" aria-modal="true" aria-labelledby="create-title" @click.self="ui.closeModal('create')">
    <form class="modal-card" @submit.prevent="submit">
      <div id="create-title" class="modal-title">Create Desktop profile</div>
      <p class="modal-lead">Create an isolated Claude Desktop environment with its own login, history, and appearance.</p>
      <label class="k" for="new-name">Name</label>
      <input id="new-name" ref="nameInput" v-model="name" class="text" placeholder="e.g. Work, Personal, Client" autocomplete="off" spellcheck="false" />
      <label class="k">Accent</label>
      <SwatchPicker v-model="color" />
      <div class="modal-actions">
        <button type="button" class="btn" @click="ui.closeModal('create')">Cancel</button>
        <button type="submit" class="primary">Create profile</button>
      </div>
    </form>
  </div>
</template>
