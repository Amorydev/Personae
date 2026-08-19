<script setup lang="ts">
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <div v-if="ui.errorMessage" class="modal" role="alertdialog" aria-modal="true" aria-labelledby="error-title" @click.self="ui.closeError">
    <div class="modal-card">
      <div id="error-title" class="modal-title">Something went wrong</div>
      <p class="hint">{{ ui.errorMessage }}</p>
      <div class="modal-actions">
        <button class="primary" @click="ui.closeError">OK</button>
      </div>
    </div>
  </div>
</template>
