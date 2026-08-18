<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useDesktopStore } from "../../stores/desktop";
import { useUiStore } from "../../stores/ui";
import { relTime } from "../../lib/format";
import Tile from "../Tile.vue";
import ThemeSwitch from "../ThemeSwitch.vue";

const desktop = useDesktopStore();
const ui = useUiStore();
const { profiles, filtered, selected, query, initialized } = storeToRefs(desktop);
</script>

<template>
  <aside class="sidebar">
    <div class="side-head" data-tauri-drag-region></div>
    <div class="search">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <circle cx="11" cy="11" r="7" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input id="search" v-model="query" placeholder="Search profiles…" autocomplete="off" spellcheck="false" />
    </div>
    <div class="section section-row">
      <span>Desktop profiles</span><span class="section-count">{{ profiles.length }}</span>
    </div>
    <ul id="list" class="profiles" role="listbox" aria-label="Desktop profiles">
      <li v-if="!initialized" class="list-loading"><span class="spinner" aria-hidden="true"></span> Loading…</li>
      <li v-else-if="!filtered.length" class="cli-list-empty">No matching profiles</li>
      <li
        v-for="p in filtered"
        :key="p.slug"
        class="p-item desktop-profile-item"
        :class="{ sel: p.slug === selected }"
        role="option"
        :aria-selected="p.slug === selected"
        :tabindex="p.slug === selected ? 0 : -1"
        :style="{ '--profile-color': p.tint || '#c2714f' }"
        @click="desktop.select(p.slug)"
        @keydown.enter.prevent="desktop.select(p.slug)"
        @keydown.space.prevent="desktop.select(p.slug)"
      >
        <Tile :tint="p.tint" :running="p.running" size="account" />
        <span class="desktop-profile-copy">
          <span class="nm">{{ p.name }}</span>
          <span class="desktop-profile-activity" :class="{ running: p.running }">
            {{ p.running ? "Running" : `Last active ${relTime(p.last_active)}` }}
          </span>
        </span>
        <span class="account-status-dot" :class="p.running ? 'is-ok' : 'is-idle'" aria-hidden="true"></span>
      </li>
    </ul>
    <div class="side-foot">
      <ThemeSwitch class="sidebar-theme-switch" />
      <button class="newbtn" @click="ui.openModal('create')"><span class="plus">+</span> New profile</button>
    </div>
  </aside>
</template>
