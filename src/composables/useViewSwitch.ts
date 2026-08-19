import { useUiStore } from "../stores/ui";
import { useCliStore } from "../stores/cli";
import { useTerminalStore } from "../stores/terminal";

// Shared by both sidebars (the Desktop/CLI switch now lives there, not in the
// detail pane's header) and App.vue's own effects.
export function useViewSwitch() {
  const ui = useUiStore();
  const cli = useCliStore();
  const terminal = useTerminalStore();

  function setView(v: "desktop" | "cli") {
    ui.setView(v);
    if (v === "cli") {
      cli.reloadCli();
      terminal.load();
    }
  }

  return { setView };
}
