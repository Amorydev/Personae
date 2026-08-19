<script setup lang="ts">
import { useThemeStore } from "../stores/theme";
import type { ThemeMode } from "../stores/theme";

const theme = useThemeStore();
const options: { id: ThemeMode; label: string }[] = [
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
  { id: "system", label: "Auto" },
];
</script>

<template>
  <div class="segmented theme-switch">
    <button
      v-for="o in options"
      :key="o.id"
      class="seg"
      :class="{ sel: theme.mode === o.id }"
      :aria-pressed="theme.mode === o.id"
      @click="theme.setMode(o.id)"
    >
      {{ o.label }}
    </button>
  </div>
</template>
