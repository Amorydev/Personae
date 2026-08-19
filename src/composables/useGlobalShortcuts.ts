import { onMounted, onUnmounted } from "vue";
import { useUiStore } from "../stores/ui";
import { useDesktopStore } from "../stores/desktop";
import { useCliStore } from "../stores/cli";

export function useGlobalShortcuts() {
  const ui = useUiStore();
  const desktop = useDesktopStore();
  const cli = useCliStore();

  function handleKeydown(e: KeyboardEvent) {
    const meta = e.metaKey || e.ctrlKey;
    const typing = ["INPUT", "TEXTAREA"].includes((document.activeElement as HTMLElement | null)?.tagName ?? "");

    if (e.key === "Escape") {
      if (ui.anyModalOpen()) ui.closeAllModals();
      return;
    }
    if (ui.anyModalOpen()) return;

    if (ui.view === "cli") {
      const interactive = (document.activeElement as HTMLElement | null)?.matches?.("button, select, [role=button]");
      if (meta && e.key.toLowerCase() === "n") { e.preventDefault(); ui.openModal("cliCreate"); return; }
      if (e.key === "/" && !typing) { e.preventDefault(); document.getElementById("cli-search")?.focus(); return; }
      if (typing || interactive) return;
      if (e.key === "ArrowDown") { e.preventDefault(); cli.moveCliSelection(1); }
      else if (e.key === "ArrowUp") { e.preventDefault(); cli.moveCliSelection(-1); }
      else if (e.key === "Enter") {
        const p = cli.cliCurrent;
        if (p) p.logged_in ? cli.doCliLaunch(p) : cli.doCliLogin();
      }
      return;
    }

    if (meta && e.key.toLowerCase() === "n") { e.preventDefault(); ui.openModal("create"); return; }
    if (meta && e.key.toLowerCase() === "l") {
      e.preventDefault();
      const p = desktop.current;
      if (p && !p.running) desktop.launch(p);
      return;
    }
    if (meta && e.key.toLowerCase() === "r") { e.preventDefault(); desktop.repair(); return; }
    if (e.key === "/" && !typing) { e.preventDefault(); document.getElementById("search")?.focus(); return; }
    if (typing) return;
    const interactive = (document.activeElement as HTMLElement | null)?.matches?.("button, select, [role=button]");
    if (interactive) return;
    if (e.key === "Enter") {
      const p = desktop.current;
      if (p) (p.running ? desktop.quit(p) : desktop.launch(p));
    } else if (e.key === "ArrowDown") { e.preventDefault(); desktop.moveSelection(1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); desktop.moveSelection(-1); }
    else if (e.key === "Backspace" || e.key === "Delete") { e.preventDefault(); ui.openModal("del"); }
  }

  onMounted(() => document.addEventListener("keydown", handleKeydown));
  onUnmounted(() => document.removeEventListener("keydown", handleKeydown));
}
