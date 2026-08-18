<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";
import { cliColor } from "../../lib/format";
import type { CliProfile } from "../../lib/types";
import ThemeSwitch from "../ThemeSwitch.vue";

const cli = useCliStore();
const ui = useUiStore();
const { cliProfiles, filteredCli, cliSelected, cliQuery, cliAvailable, initialized } = storeToRefs(cli);

function identity(p: CliProfile): string {
  if (!p.logged_in) return "Needs login";
  if (p.auth_mode === "api_key") return p.provider_model || "API key";
  return p.account_email || "Signed in";
}
</script>

<template>
  <aside class="sidebar">
    <div class="side-head" data-tauri-drag-region></div>
    <div class="search cli-search">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
        <circle cx="11" cy="11" r="7" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input id="cli-search" v-model="cliQuery" placeholder="Search accounts…" autocomplete="off" spellcheck="false" />
    </div>
    <div class="section section-row">
      <span>CLI accounts</span><span class="section-count">{{ cliProfiles.length }}</span>
    </div>
    <ul id="cli-list" class="profiles" role="listbox" aria-label="CLI accounts">
      <li v-if="!initialized" class="list-loading"><span class="spinner" aria-hidden="true"></span> Loading…</li>
      <li v-else-if="!filteredCli.length" class="cli-list-empty">No matching accounts</li>
      <li
        v-for="p in filteredCli"
        :key="p.slug"
        class="p-item cli-account-item"
        :class="{ sel: p.slug === cliSelected }"
        role="option"
        :aria-selected="p.slug === cliSelected"
        :tabindex="p.slug === cliSelected ? 0 : -1"
        @click="cli.selectCliProfile(p)"
        @keydown.enter.prevent="cli.selectCliProfile(p)"
        @keydown.space.prevent="cli.selectCliProfile(p)"
      >
        <span class="cli-mark sm" :style="{ '--account-color': cliColor(p) }"><span aria-hidden="true">&gt;_</span></span>
        <span class="cli-account-copy">
          <span class="nm">{{ p.name }}</span>
          <span class="cli-account-identity" :class="{ 'needs-login': !p.logged_in }">{{ identity(p) }}</span>
        </span>
        <span class="account-status-dot" :class="p.logged_in ? 'is-ok' : 'is-warn'" :title="p.logged_in ? 'Signed in' : 'Needs login'" aria-hidden="true"></span>
      </li>
    </ul>
    <div class="side-foot">
      <ThemeSwitch class="sidebar-theme-switch" />
      <button class="settings-btn" @click="ui.openModal('settings')">⚙ Settings</button>
      <button id="cli-new-side" class="newbtn" :disabled="!cliAvailable" @click="ui.openModal('cliCreate')">
        <span class="plus">+</span> New account
      </button>
    </div>
  </aside>
</template>
