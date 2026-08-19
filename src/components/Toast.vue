<script setup lang="ts">
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <div id="app-toast" class="app-toast" :class="[ui.toast.tone, { visible: ui.toast.visible }]" role="status" aria-live="polite">
    {{ ui.toast.message }}
  </div>
</template>
