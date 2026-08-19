<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useCliStore } from "../../stores/cli";
import { useUiStore } from "../../stores/ui";
import { cliColor, prettySize, relTime, createdLabel, sessionExpiry, usageLabel } from "../../lib/format";
import { invoke } from "../../lib/tauri";
import WorkspaceList from "./WorkspaceList.vue";
import LoadingState from "../LoadingState.vue";

const cli = useCliStore();
const ui = useUiStore();
const { cliAvailable, cliCurrent, cliWorkspaces, initialized } = storeToRefs(cli);

const expiry = computed(() =>
  cliCurrent.value?.auth_mode === "oauth" ? sessionExpiry(cliCurrent.value.refresh_expires_at) : null
);
// The metrics column is already labeled "Session expires"; drop the
// redundant "Signed in until "/"Session " prefix sessionExpiry() uses
// for the standalone-pill phrasing.
const expiryColumnValue = computed(() => {
  const e = expiry.value;
  if (!e) return "";
  return e.label.replace(/^Signed in until /, "").replace(/^Session /, "");
});
// A live 401 from the usage endpoint (see cli::fetch_live_usage) means
// Anthropic just told us directly the stored token is dead — treat that the
// same as "not logged in" here even though the on-disk credential file is
// still present, rather than waiting for the next full reloadCli() to notice.
const sessionExpired = computed(() => !!cliCurrent.value && cli.sessionExpiredSlugs.has(cliCurrent.value.slug));
const effectivelyLoggedIn = computed(() => !!cliCurrent.value?.logged_in && !sessionExpired.value);

// One status pill, not two redundant "you're signed in" badges — expiry gets
// its own metrics column instead (see cli-metrics below).
const statusPill = computed(() => ({
  label: sessionExpired.value ? "Session expired" : effectivelyLoggedIn.value ? "Signed in" : "Needs login",
  warn: !effectivelyLoggedIn.value,
}));

function reveal() {
  if (cliCurrent.value) invoke("reveal_path", { path: cliCurrent.value.config_dir });
}
function openIde() {
  ui.openModal("ide");
}
</script>

<template>
  <LoadingState v-if="!initialized">Loading CLI accounts…</LoadingState>
  <div v-else-if="!cliAvailable" class="empty cli-empty">
    <div class="empty-mark">&gt;_</div>
    <div class="big">Claude Code is required</div>
    <div class="small">Install the <code>claude</code> CLI to create isolated accounts, then reopen Personae.</div>
  </div>
  <div v-else-if="!cliCurrent" class="empty cli-empty">
    <div class="empty-mark">&gt;_</div>
    <div class="big">Create your first CLI account</div>
    <div class="small">Each account keeps its own Claude login, history, and project environments.</div>
    <button class="primary" @click="ui.openModal('cliCreate')">Create account</button>
  </div>
  <div v-else class="cli-content">
    <section class="cli-profile-hero">
      <div class="cli-hero-identity">
        <span class="cli-mark lg" :style="{ '--account-color': cliColor(cliCurrent) }"><span aria-hidden="true">&gt;_</span></span>
        <div class="cli-hero-copy">
          <div class="cli-title-line">
            <h1>{{ cliCurrent.name }}</h1>
          </div>
          <p>
            {{
              cliCurrent.auth_mode === "api_key"
                ? `API key${cliCurrent.provider_model ? " · " + cliCurrent.provider_model : ""}`
                : cliCurrent.account_email || (cliCurrent.logged_in ? "Claude account connected" : "Sign in once to activate this environment")
            }}
          </p>
        </div>
      </div>
      <div class="cli-hero-actions">
        <template v-if="effectivelyLoggedIn">
          <button v-if="expiry?.warn" class="btn" @click="cli.doCliLogin">Relogin</button>
          <button class="primary" @click="cli.doCliLaunch(cliCurrent)"><span aria-hidden="true">↗</span> Launch CLI</button>
          <button class="btn" @click="openIde">Open project…</button>
        </template>
        <template v-else>
          <button class="primary" @click="cli.doCliLogin">{{ sessionExpired ? "Sign in again" : "Log in account" }}</button>
          <button class="btn" disabled title="Log in first">Launch CLI</button>
        </template>
      </div>
    </section>
    <section v-if="cliCurrent.logged_in" class="cli-metrics" aria-label="Account overview">
      <div><span>Status</span><strong class="metric-with-dot"><span class="account-status-dot" :class="effectivelyLoggedIn ? 'is-ok' : 'is-idle'" aria-hidden="true"></span>{{ statusPill.label }}</strong></div>
      <div><span>Storage</span><strong>{{ prettySize(cliCurrent.data_size) }}</strong></div>
      <div><span>Last active</span><strong>{{ relTime(cliCurrent.last_active) }}</strong></div>
      <div><span>Created</span><strong>{{ createdLabel(cliCurrent.created) }}</strong></div>
      <div><span>Session expires</span><strong :class="{ 'text-warn': expiry?.warn }">{{ expiry ? expiryColumnValue : "—" }}</strong></div>
      <div>
        <span class="usage-label-row">
          Usage
          <button
            class="mini usage-refresh-btn"
            :class="{ spinning: cli.usageRefreshingSlug === cliCurrent.slug }"
            title="Refresh usage from Anthropic"
            aria-label="Refresh usage from Anthropic"
            @click="cli.refreshUsageLive(cliCurrent)"
          >↻</button>
        </span>
        <strong>{{ usageLabel(cliCurrent.session_usage_pct, cliCurrent.weekly_usage_pct, cliCurrent.subscription_type, cliCurrent.rate_limit_tier) }}</strong>
      </div>
    </section>
    <section v-else class="cli-setup-card">
      <div class="setup-icon" aria-hidden="true">↳</div>
      <div>
        <h2>Finish setting up this account</h2>
        <p>Personae will open Claude's secure sign-in in Terminal. When it is complete, return here to launch or connect projects.</p>
      </div>
    </section>
    <section class="cli-section">
      <div class="section-heading">
        <div>
          <h2>Workspaces</h2>
          <p>Projects that reopen with this account active.</p>
        </div>
      </div>
      <WorkspaceList
        :workspaces="cliWorkspaces.filter((w) => w.account_slug === cliCurrent!.slug)"
        :logged-in="cliCurrent.logged_in"
        @open-project="openIde"
      />
    </section>
    <details class="cli-advanced">
      <summary>
        <span><strong>Advanced details</strong><small>Config, login, and account actions</small></span>
        <span class="chevron" aria-hidden="true">⌄</span>
      </summary>
      <div class="advanced-body">
        <div class="advanced-path-row">
          <div><span class="advanced-label">Config directory</span><code>{{ cliCurrent.config_dir }}</code></div>
          <button class="btn compact" @click="reveal">Show in Finder</button>
        </div>
        <div class="advanced-path-row">
          <div>
            <span class="advanced-label">Provider</span>
            <code>{{ cliCurrent.auth_mode === "api_key" ? `API key${cliCurrent.provider_model ? " · " + cliCurrent.provider_model : ""}` : "claude.ai sign-in" }}</code>
          </div>
          <button class="btn compact" @click="ui.openModal('provider')">Change…</button>
        </div>
        <div class="advanced-actions">
          <button v-if="cliCurrent.logged_in && cliCurrent.auth_mode === 'oauth'" class="text-btn" @click="cli.doCliLogin">Switch or refresh login</button>
          <button class="danger-ghost" @click="ui.openModal('cliDel')">Delete account…</button>
        </div>
      </div>
    </details>
  </div>
</template>
