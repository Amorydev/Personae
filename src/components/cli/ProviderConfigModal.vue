<script setup lang="ts">
import { ref, watch } from "vue";
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";
import type { AuthMode } from "../../lib/types";

const cli = useCliStore();
const ui = useUiStore();

const authMode = ref<AuthMode>("oauth");
const baseUrl = ref("");
const apiKey = ref("");
const model = ref("");
const smallFastModel = ref("");
const revealKey = ref(false);

watch(
  () => ui.modals.provider,
  async (open) => {
    if (!open) return;
    const p = cli.cliCurrent;
    if (!p) { ui.closeModal("provider"); return; }
    const cfg = await cli.getProviderConfig(p.name);
    authMode.value = cfg.auth_mode;
    baseUrl.value = cfg.base_url ?? "";
    apiKey.value = cfg.api_key ?? "";
    model.value = cfg.model ?? "";
    smallFastModel.value = cfg.small_fast_model ?? "";
    revealKey.value = false;
  }
);

async function submit() {
  const p = cli.cliCurrent;
  if (!p) return;
  try {
    await cli.setProviderConfig(p.name, {
      auth_mode: authMode.value,
      base_url: baseUrl.value || null,
      api_key: apiKey.value || null,
      model: model.value || null,
      small_fast_model: smallFastModel.value || null,
    });
  } catch (e) {
    alert(String(e));
    return;
  }
  ui.closeModal("provider");
  if (authMode.value === "oauth") await cli.doCliLogin();
}
</script>

<template>
  <div v-if="ui.modals.provider" class="modal" role="dialog" aria-modal="true" aria-labelledby="provider-title" @click.self="ui.closeModal('provider')">
    <form class="modal-card" @submit.prevent="submit">
      <div id="provider-title" class="modal-title">Provider</div>
      <p class="modal-lead">Sign in with claude.ai, or point this account at an API key — Anthropic, or any Anthropic-API-compatible endpoint (a custom base URL and model, e.g. a DeepSeek proxy).</p>

      <div class="segmented">
        <button type="button" class="seg" :class="{ sel: authMode === 'oauth' }" @click="authMode = 'oauth'">Sign in with claude.ai</button>
        <button type="button" class="seg" :class="{ sel: authMode === 'api_key' }" @click="authMode = 'api_key'">Use an API key</button>
      </div>

      <template v-if="authMode === 'api_key'">
        <div class="field-group">
          <label class="k" for="provider-key">API key / token</label>
          <div class="key-row">
            <input id="provider-key" v-model="apiKey" class="text" :type="revealKey ? 'text' : 'password'" placeholder="sk-…" autocomplete="off" spellcheck="false" />
            <button type="button" class="btn compact" @click="revealKey = !revealKey">{{ revealKey ? "Hide" : "Show" }}</button>
          </div>
        </div>
        <div class="field-group">
          <label class="k" for="provider-base-url">Base URL</label>
          <input id="provider-base-url" v-model="baseUrl" class="text" placeholder="https://api.example.com" autocomplete="off" spellcheck="false" />
        </div>
        <div class="field-group">
          <label class="k" for="provider-model">Model</label>
          <input id="provider-model" v-model="model" class="text" placeholder="e.g. deepseek-chat, claude-fable-5" autocomplete="off" spellcheck="false" />
        </div>
        <div class="field-group">
          <label class="k" for="provider-small-model">Small / fast model (optional)</label>
          <input id="provider-small-model" v-model="smallFastModel" class="text" autocomplete="off" spellcheck="false" />
        </div>
      </template>
      <p v-else class="hint">This account signs in through Claude Code's normal claude.ai flow — no key stored here.</p>

      <div class="modal-actions">
        <button type="button" class="btn" @click="ui.closeModal('provider')">Cancel</button>
        <button type="submit" class="primary">Save</button>
      </div>
    </form>
  </div>
</template>
