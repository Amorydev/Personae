<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useDesktopStore } from "../../stores/desktop";
import { useUiStore } from "../../stores/ui";
import { prettySize, relTime, createdLabel } from "../../lib/format";
import { invoke } from "../../lib/tauri";
import Tile from "../Tile.vue";
import LoadingState from "../LoadingState.vue";

const desktop = useDesktopStore();
const ui = useUiStore();
const { current, initialized } = storeToRefs(desktop);

function primary() {
  if (!current.value) return;
  current.value.running ? desktop.quit(current.value) : desktop.launch(current.value);
}
function reveal() {
  if (current.value) invoke("reveal_path", { path: current.value.data_path });
}
function feedback() {
  invoke("open_url", { url: "mailto:amory.dev@gmail.com?subject=Personae%20feedback" });
}
</script>

<template>
  <LoadingState v-if="!initialized">Loading Desktop profiles…</LoadingState>
  <div v-else-if="!current" class="empty cli-empty">
    <div class="empty-mark">✦</div>
    <div class="big">Create your first Desktop profile</div>
    <div class="small">Each profile keeps its own Claude login, history, data, and appearance.</div>
    <button class="primary" @click="ui.openModal('create')">Create profile</button>
  </div>
  <div v-else class="desktop-content">
    <section class="cli-profile-hero">
      <div class="cli-hero-identity">
        <Tile :tint="current.tint" :running="current.running" size="md" />
        <div class="cli-hero-copy">
          <div class="cli-title-line">
            <h1>{{ current.name }}</h1>
            <span class="status-pill" :class="current.running ? 'running' : 'ready'">
              <span></span>{{ current.running ? "Running" : "Ready" }}
            </span>
          </div>
          <p>{{ current.running ? "Claude Desktop is active in this profile" : `Last active ${relTime(current.last_active)}` }}</p>
        </div>
      </div>
      <div class="cli-hero-actions">
        <button class="primary" @click="primary">
          <span aria-hidden="true">{{ current.running ? "■" : "↗" }}</span> {{ current.running ? "Quit Desktop" : "Launch Desktop" }}
        </button>
        <button class="btn" @click="reveal">Show data…</button>
      </div>
    </section>
    <section class="cli-metrics" aria-label="Profile overview">
      <div><span>Status</span><strong class="metric-with-dot"><span class="account-status-dot" :class="current.running ? 'is-ok' : 'is-idle'" aria-hidden="true"></span>{{ current.running ? "Running" : "Ready" }}</strong></div>
      <div><span>Storage</span><strong>{{ prettySize(current.data_size) }}</strong></div>
      <div><span>Last active</span><strong>{{ relTime(current.last_active) }}</strong></div>
      <div><span>Created</span><strong>{{ createdLabel(current.created) }}</strong></div>
      <div>
        <span class="metric-label-row">
          Session
          <button
            class="mini usage-refresh-btn"
            :class="{ spinning: desktop.usageRefreshingSlug === current.slug }"
            title="Refresh usage from the desktop app"
            aria-label="Refresh usage from the desktop app"
            @click="desktop.refreshUsageLive(current)"
          >↻</button>
        </span>
        <strong>{{ current.session_usage_pct != null ? `${current.session_usage_pct}%` : "—" }}</strong>
      </div>
      <div><span>This week</span><strong>{{ current.weekly_usage_pct != null ? `${current.weekly_usage_pct}%` : "—" }}</strong></div>
    </section>
    <section class="cli-section">
      <div class="section-heading">
        <div>
          <h2>Profile data</h2>
          <p>Login, history, cache, and settings isolated for this profile.</p>
        </div>
      </div>
      <div class="profile-data-card">
        <span class="profile-data-icon" aria-hidden="true">◇</span>
        <span class="profile-data-copy"><strong>Claude Desktop data</strong><code>{{ current.data_path }}</code></span>
        <button class="btn compact" @click="reveal">Show in Finder</button>
      </div>
    </section>
    <details class="cli-advanced desktop-advanced">
      <summary>
        <span><strong>Advanced details</strong><small>Appearance, repair, and profile actions</small></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="advanced-body">
        <div class="desktop-accent-row">
          <span class="accent-preview" :style="{ '--preview-color': current.tint || '#c2714f' }"></span>
          <div><strong>Profile accent</strong><span>Used for this profile's icon and actions.</span></div>
          <button class="btn compact" @click="ui.openModal('edit')">Change…</button>
        </div>
        <div class="advanced-actions desktop-utility-actions">
          <button class="text-btn" @click="desktop.repair">Repair launchers</button>
          <button class="text-btn" @click="feedback">Send feedback</button>
          <button class="danger-ghost" @click="ui.openModal('del')">Delete profile…</button>
        </div>
      </div>
    </details>
  </div>
</template>
