<script setup lang="ts">
import { ref, watch } from "vue";
import { useDesktopStore } from "../../stores/desktop";
import { useUiStore } from "../../stores/ui";

const desktop = useDesktopStore();
const ui = useUiStore();
const purge = ref(false);
const pendingName = ref<string | null>(null);

watch(
  () => ui.modals.del,
  (open) => {
    if (!open) return;
    if (!desktop.current) {
      ui.closeModal("del");
      return;
    }
    pendingName.value = desktop.current.name;
    purge.value = false;
  }
);

async function confirmDelete() {
  if (!pendingName.value) return;
  const name = pendingName.value;
  ui.closeModal("del");
  pendingName.value = null;
  try {
    await desktop.remove(name, purge.value);
  } catch {
    // mirrors the vanilla implementation's silent catch
  }
}
</script>

<template>
  <div v-if="ui.modals.del" class="modal" role="dialog" aria-modal="true" aria-labelledby="del-title" @click.self="ui.closeModal('del')">
    <div class="modal-card">
      <div id="del-title" class="modal-title">Delete "{{ pendingName }}"?</div>
      <label class="chk"><input v-model="purge" type="checkbox" /> Also erase login &amp; history data (cannot be undone)</label>
      <div class="modal-actions">
        <button class="btn" @click="ui.closeModal('del')">Cancel</button>
        <button class="danger" @click="confirmDelete">Delete profile</button>
      </div>
    </div>
  </div>
</template>
