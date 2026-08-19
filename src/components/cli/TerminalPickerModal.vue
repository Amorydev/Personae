<script setup lang="ts">
import { ref, watch } from "vue";
import { useTerminalStore } from "../../stores/terminal";
import { useUiStore } from "../../stores/ui";
import { useCliStore } from "../../stores/cli";

const terminal = useTerminalStore();
const ui = useUiStore();
const cli = useCliStore();
const selected = ref("");

watch(
  () => ui.modals.terminal,
  async (open) => {
    if (!open) return;
    if (!terminal.terminals.length) await terminal.load();
    selected.value = terminal.defaultTerminalId || terminal.terminals[0]?.id || "";
  }
);

function customLabel(): string {
  if (!terminal.customPath) return "Custom program…";
  const name = terminal.customPath.split(/[\\/]/).pop() || terminal.customPath;
  return `Custom: ${name}`;
}

async function pickCustom() {
  const path = await terminal.pickCustom();
  if (path) selected.value = "custom";
}

async function confirm() {
  if (!selected.value) return;
  if (selected.value === "custom") {
    if (!terminal.customPath) return;
    await terminal.chooseCustom(terminal.customPath);
  } else {
    await terminal.choose(selected.value);
  }
  ui.closeModal("terminal");
  await cli.resumePendingLaunch();
}

function cancel() {
  ui.closeModal("terminal");
  cli.pendingLaunch = null;
}
</script>

<template>
  <div v-if="ui.modals.terminal" class="modal" role="dialog" aria-modal="true" aria-labelledby="terminal-modal-title" @click.self="cancel">
    <div class="modal-card">
      <div id="terminal-modal-title" class="modal-title">Choose your terminal</div>
      <p class="modal-lead">Personae will open CLI accounts in this terminal from now on. Change it anytime from the CLI sidebar.</p>
      <div class="terminal-options">
        <label v-for="t in terminal.terminals" :key="t.id" class="chk">
          <input v-model="selected" type="radio" name="terminal" :value="t.id" /> {{ t.name }}
        </label>
        <label class="chk">
          <input type="radio" name="terminal" value="custom" :checked="selected === 'custom'" @click.prevent="pickCustom" />
          {{ customLabel() }}
        </label>
      </div>
      <div class="modal-actions">
        <button class="btn" @click="cancel">Cancel</button>
        <button class="primary" @click="confirm">Continue</button>
      </div>
    </div>
  </div>
</template>
