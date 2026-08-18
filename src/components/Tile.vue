<script setup lang="ts">
withDefaults(defineProps<{ tint?: string | null; running?: boolean; size?: string }>(), {
  tint: null,
  running: false,
  size: "md",
});

const rays = Array.from({ length: 12 }, (_, i) => (360 / 12) * i);
</script>

<template>
  <div class="tile" :class="size" :style="{ '--tint': tint || '#8a8a8a' }">
    <svg class="burst" viewBox="0 0 100 100" aria-hidden="true">
      <g fill="currentColor">
        <rect v-for="a in rays" :key="a" x="46" y="7" width="8" height="34" rx="4" :transform="`rotate(${a} 50 50)`" />
      </g>
    </svg>
    <span v-if="running" class="rundot"></span>
  </div>
</template>
