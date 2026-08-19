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
  <div v-if="!workspaces.length" class="workspace-empty">
    <span class="workspace-empty-icon" aria-hidden="true">◇</span>
    <div>
      <strong>No projects yet</strong>
      <p>{{ loggedIn ? "Open a project to bind it to this account." : "Log in before connecting a project." }}</p>
    </div>
    <button v-if="loggedIn" class="btn compact" @click="$emit('open-project')">Open project…</button>
  </div>
  <div v-else class="workspace-list">
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
    <button v-if="loggedIn" class="workspace-add-row" @click="$emit('open-project')">
      <span aria-hidden="true">◇</span> Open project…
    </button>
  </div>
</template>
