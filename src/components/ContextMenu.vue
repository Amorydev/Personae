<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";

export interface ContextMenuItem {
  label: string;
  action: () => void;
  danger?: boolean;
}

const props = defineProps<{ x: number; y: number; items: ContextMenuItem[] }>();
const emit = defineEmits<{ (e: "close"): void }>();

function run(item: ContextMenuItem) {
  item.action();
  emit("close");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onUnmounted(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="context-menu-backdrop" @click="emit('close')" @contextmenu.prevent="emit('close')">
    <div
      class="context-menu"
      role="menu"
      :style="{ left: `${props.x}px`, top: `${props.y}px` }"
      @click.stop
    >
      <button v-for="item in items" :key="item.label" role="menuitem" class="context-menu-item" :class="{ danger: item.danger }" @click="run(item)">
        {{ item.label }}
      </button>
    </div>
  </div>
</template>
