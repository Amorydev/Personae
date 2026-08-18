<script setup lang="ts">
import { ref, watch } from "vue";
import { useDesktopStore } from "../../stores/desktop";
import { useUiStore } from "../../stores/ui";
import { PALETTE } from "../../lib/format";
import SwatchPicker from "../SwatchPicker.vue";

const desktop = useDesktopStore();
const ui = useUiStore();
const color = ref(PALETTE[0]);

watch(
  () => ui.modals.edit,
  (open) => {
    if (!open) return;
    color.value = (desktop.current?.tint || `#${PALETTE[0]}`).replace("#", "");
  }
);

async function pick(hex: string) {
  color.value = hex;
  const p = desktop.current;
  if (!p) return;
  try {
    await desktop.setColor(p.name, hex);
  } catch {
    // best-effort, mirrors the vanilla implementation's silent catch
  }
  ui.closeModal("edit");
}
</script>

<template>
  <div v-if="ui.modals.edit" class="modal" role="dialog" aria-modal="true" aria-labelledby="edit-title" @click.self="ui.closeModal('edit')">
    <div class="modal-card">
      <div id="edit-title" class="modal-title">Change accent</div>
      <label class="k">Pick a colour — applies immediately</label>
      <SwatchPicker :model-value="color" @update:model-value="pick" />
      <div class="modal-actions">
        <button class="btn" @click="ui.closeModal('edit')">Done</button>
      </div>
    </div>
  </div>
</template>
