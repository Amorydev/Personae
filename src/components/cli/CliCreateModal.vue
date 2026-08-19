<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";
import type { AuthMode } from "../../lib/types";

const cli = useCliStore();
const ui = useUiStore();
const name = ref("");
const nameInput = ref<HTMLInputElement | null>(null);

const authMode = ref<AuthMode>("oauth");
const baseUrl = ref("");
const apiKey = ref("");
const model = ref("");
const smallFastModel = ref("");
const revealKey = ref(false);

watch(
  () => ui.modals.cliCreate,
  (open) => {
    if (!open) return;
    name.value = "";
    authMode.value = "oauth";
    baseUrl.value = "";
    apiKey.value = "";
    model.value = "";
    smallFastModel.value = "";
    revealKey.value = false;
    nextTick(() => nameInput.value?.focus());
  }
);

async function submit() {
  const n = name.value.trim();
  if (!n) return;
  try {
    await cli.doCliCreate(
      n,
      authMode.value === "api_key"
        ? { auth_mode: "api_key", base_url: baseUrl.value || null, api_key: apiKey.value || null, model: model.value || null, small_fast_model: smallFastModel.value || null }
        : { auth_mode: "oauth", base_url: null, api_key: null, model: null, small_fast_model: null }
    );
  } catch (e) {
    ui.showToast(String(e), "error");
    return;
  }
  ui.closeModal("cliCreate");
}
</script>

<template>
  <div v-if="ui.modals.cliCreate" class="modal" role="dialog" aria-modal="true" aria-labelledby="cli-create-title" @click.self="ui.closeModal('cliCreate')">
    <form class="modal-card cli-create-card" @submit.prevent="submit">
      <div id="cli-create-title" class="modal-title">Create CLI account</div>
      <p class="modal-lead">Give this environment a memorable name. Its Claude login and project history stay isolated.</p>
      <label class="k" for="cli-new-name">Name</label>
      <input id="cli-new-name" ref="nameInput" v-model="name" class="text" placeholder="e.g. Work, Personal, Client" autocomplete="off" spellcheck="false" />

      <label class="k">Sign-in</label>
      <div class="segmented">
        <button type="button" class="seg" :class="{ sel: authMode === 'oauth' }" @click="authMode = 'oauth'">Sign in with claude.ai</button>
        <button type="button" class="seg" :class="{ sel: authMode === 'api_key' }" @click="authMode = 'api_key'">Use an API key</button>
      </div>

      <template v-if="authMode === 'api_key'">
        <div class="field-group">
          <label class="k" for="cli-new-key">API key / token</label>
          <div class="key-row">
            <input id="cli-new-key" v-model="apiKey" class="text" :type="revealKey ? 'text' : 'password'" placeholder="sk-…" autocomplete="off" spellcheck="false" />
            <button type="button" class="btn compact" @click="revealKey = !revealKey">{{ revealKey ? "Hide" : "Show" }}</button>
          </div>
        </div>
        <div class="field-group">
          <label class="k" for="cli-new-base-url">Base URL</label>
          <input id="cli-new-base-url" v-model="baseUrl" class="text" placeholder="https://api.example.com" autocomplete="off" spellcheck="false" />
        </div>
        <div class="field-group">
          <label class="k" for="cli-new-model">Model</label>
          <input id="cli-new-model" v-model="model" class="text" placeholder="e.g. deepseek-chat, claude-fable-5" autocomplete="off" spellcheck="false" />
        </div>
        <div class="field-group">
          <label class="k" for="cli-new-small-model">Small / fast model (optional)</label>
          <input id="cli-new-small-model" v-model="smallFastModel" class="text" autocomplete="off" spellcheck="false" />
        </div>
      </template>

      <div class="modal-actions">
        <button type="button" class="btn" @click="ui.closeModal('cliCreate')">Cancel</button>
        <button type="submit" class="primary">Create account</button>
      </div>
    </form>
  </div>
</template>
