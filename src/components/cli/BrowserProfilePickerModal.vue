<script setup lang="ts">
import { ref, watch } from "vue";
import { useBrowserStore } from "../../stores/browser";
import { useUiStore } from "../../stores/ui";
import { useCliStore } from "../../stores/cli";

const browser = useBrowserStore();
const ui = useUiStore();
const cli = useCliStore();
const selected = ref("");

watch(
  () => ui.modals.browserProfile,
  async (open) => {
    if (!open) return;
    if (!browser.profilesLoaded) await browser.loadProfiles();
    const slug = cli.pendingLogin?.slug;
    // Preselect whatever this account used last; failing that, the first
    // profile, so Continue is always a valid action.
    selected.value =
      (slug ? await browser.accountProfile(slug) : null) || browser.profiles[0]?.dir || "";
  }
);

function subtitle(account: string | null, dir: string): string {
  return account || dir;
}

async function confirm() {
  const p = cli.pendingLogin;
  if (!selected.value || !p) return;
  await browser.setAccountProfile(p.slug, selected.value);
  ui.closeModal("browserProfile");
  await cli.resumePendingLogin();
}

function cancel() {
  ui.closeModal("browserProfile");
  cli.pendingLogin = null;
}
</script>

<template>
  <div
    v-if="ui.modals.browserProfile"
    class="modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="browser-profile-modal-title"
    @click.self="cancel"
  >
    <div class="modal-card">
      <div id="browser-profile-modal-title" class="modal-title">Choose a browser profile</div>
      <p class="modal-lead">
        Sign-in opens in your browser. Pick the profile that is already signed in to Claude
        for<template v-if="cli.pendingLogin"> &ldquo;{{ cli.pendingLogin.name }}&rdquo;</template> —
        otherwise you land on the login page instead of the Authorize button. Personae
        remembers this per account.
      </p>
      <div class="terminal-options">
        <label v-for="p in browser.profiles" :key="p.dir" class="chk">
          <input v-model="selected" type="radio" name="browser-profile" :value="p.dir" />
          {{ p.name }} <span class="muted">({{ subtitle(p.account, p.dir) }})</span>
        </label>
      </div>
      <div class="modal-actions">
        <button class="btn" @click="cancel">Cancel</button>
        <button class="primary" @click="confirm">Continue</button>
      </div>
    </div>
  </div>
</template>
