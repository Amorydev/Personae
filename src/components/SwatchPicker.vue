<script setup lang="ts">
import { computed } from "vue";
import { PALETTE } from "../lib/format";

const props = withDefaults(defineProps<{ modelValue?: string }>(), { modelValue: PALETTE[0] });
const emit = defineEmits<{ (e: "update:modelValue", hex: string): void }>();

const sel = computed(() => (props.modelValue || PALETTE[0]).toUpperCase());

function pick(hex: string) {
  emit("update:modelValue", hex);
}
</script>

<template>
  <div class="swatches">
    <button
      v-for="hex in PALETTE"
      :key="hex"
      type="button"
      class="sw"
      :class="{ sel: hex.toUpperCase() === sel }"
      :aria-label="`Accent #${hex}`"
      :aria-pressed="hex.toUpperCase() === sel"
      :title="`#${hex}`"
      :style="{ background: `#${hex}` }"
      @click="pick(hex)"
    />
  </div>
</template>
