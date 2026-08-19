<script setup lang="ts">
import type { Workspace } from "../../lib/types";
import { relTime } from "../../lib/format";
import { useCliStore } from "../../stores/cli";

defineProps<{ workspaces: Workspace[]; loggedIn: boolean }>();
defineEmits<{ (e: "open-project"): void }>();
const cli = useCliStore();

function folderName(path: string): string {
  return path.replace(/\/+$/, "").split("/").pop() || path;
}

function remove(id: string, e: MouseEvent) {
  e.stopPropagation();
  cli.deleteWorkspace(id);
}
</script>

<template>
  <div class="workspace-list">
    <div v-for="w in workspaces" :key="w.id" class="workspace-row">
      <button
        class="workspace-open"
        :title="`${w.project_path} — open in ${w.ide_name}`"
        :aria-label="`Open ${folderName(w.project_path)} in ${w.ide_name}`"
        @click="cli.openWorkspace(w.id)"
      >
        <span class="workspace-icon">◇</span>
        <span class="workspace-copy">
          <strong>{{ folderName(w.project_path) }}</strong>
          <span>{{ w.project_path }}</span>
        </span>
        <span class="workspace-meta">
          <span class="ide-badge">{{ w.ide_name }}</span>
          <span>{{ relTime(w.last_opened) }}</span>
        </span>
      </button>
      <button class="workspace-remove" title="Remove this workspace" :aria-label="`Remove ${folderName(w.project_path)}`" @click="remove(w.id, $event)">×</button>
    </div>
    <!-- Same row format as the real entries above (icon + copy), so the
         add-project affordance doesn't look like a different kind of thing
         — shown whether or not workspaces exist, dashed to read as "add". -->
    <div v-if="loggedIn" class="workspace-row workspace-row-add">
      <button class="workspace-open" @click="$emit('open-project')">
        <span class="workspace-icon" aria-hidden="true">◇</span>
        <span class="workspace-copy">
          <strong>Open project…</strong>
          <span>Bind another project to this account</span>
        </span>
      </button>
    </div>
    <div v-else class="workspace-row workspace-row-add">
      <button class="workspace-open" disabled>
        <span class="workspace-icon" aria-hidden="true">◇</span>
        <span class="workspace-copy">
          <strong>Log in first</strong>
          <span>Sign in before connecting a project</span>
        </span>
      </button>
    </div>
  </div>
</template>
