import { onMounted, onUnmounted } from "vue";
import { useUiStore } from "../stores/ui";
import { useDesktopStore } from "../stores/desktop";
import { useCliStore } from "../stores/cli";

export const FOCUS_THROTTLE_MS = 800;
export const DESKTOP_POLL_MS = 6000;

export function useStateRefresh() {
  const ui = useUiStore();
  const desktop = useDesktopStore();
  const cli = useCliStore();

  let lastFocusReload = 0;
  let poll: number | undefined;

  function handleFocus() {
    const t = Date.now();
    if (t - lastFocusReload < FOCUS_THROTTLE_MS) return;
    lastFocusReload = t;
    if (ui.view === "cli") cli.reloadCli();
    else desktop.reload();
  }

  function tick() {
    if (document.hidden || ui.view === "cli") return;
    desktop.reload();
  }

  onMounted(() => {
    window.addEventListener("focus", handleFocus);
    poll = window.setInterval(tick, DESKTOP_POLL_MS);
  });

  onUnmounted(() => {
    window.removeEventListener("focus", handleFocus);
    if (poll !== undefined) window.clearInterval(poll);
  });

  return { handleFocus, tick };
}
