import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { defineComponent } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DESKTOP_POLL_MS, useStateRefresh } from "./useStateRefresh";
import { useUiStore } from "../stores/ui";
import { useDesktopStore } from "../stores/desktop";
import { useCliStore } from "../stores/cli";

type Refresh = ReturnType<typeof useStateRefresh>;

function setup() {
  const ui = useUiStore();
  const desktop = useDesktopStore();
  const cli = useCliStore();
  desktop.reload = vi.fn().mockResolvedValue(undefined);
  cli.reloadCli = vi.fn().mockResolvedValue(undefined);

  let refresh!: Refresh;
  const wrapper = mount(
    defineComponent({
      setup() {
        refresh = useStateRefresh();
        return () => null;
      },
    }),
  );

  return { ui, desktop, cli, wrapper, ...refresh };
}

describe("useStateRefresh", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  it("refreshes the desktop view on focus", () => {
    const { ui, desktop, handleFocus } = setup();
    ui.view = "desktop";
    handleFocus();
    expect(desktop.reload).toHaveBeenCalledTimes(1);
  });

  it("refreshes the cli view on focus instead of the desktop one", () => {
    const { ui, desktop, cli, handleFocus } = setup();
    ui.view = "cli";
    handleFocus();
    expect(cli.reloadCli).toHaveBeenCalledTimes(1);
    expect(desktop.reload).not.toHaveBeenCalled();
  });

  it("throttles rapid focus events", () => {
    const { ui, desktop, handleFocus } = setup();
    ui.view = "desktop";
    handleFocus();
    handleFocus();
    handleFocus();
    expect(desktop.reload).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(900);
    handleFocus();
    expect(desktop.reload).toHaveBeenCalledTimes(2);
  });

  it("polls the desktop view while it is visible", () => {
    const { ui, desktop, tick } = setup();
    ui.view = "desktop";
    tick();
    expect(desktop.reload).toHaveBeenCalledTimes(1);
  });

  it("skips polling while the window is hidden", () => {
    const { ui, desktop, tick } = setup();
    ui.view = "desktop";
    const hidden = vi.spyOn(document, "hidden", "get").mockReturnValue(true);
    tick();
    expect(desktop.reload).not.toHaveBeenCalled();
    hidden.mockRestore();
  });

  it("polls on an interval once mounted", () => {
    const { ui, desktop } = setup();
    ui.view = "desktop";
    expect(desktop.reload).not.toHaveBeenCalled();

    vi.advanceTimersByTime(DESKTOP_POLL_MS * 2);
    expect(desktop.reload).toHaveBeenCalledTimes(2);
  });

  it("stops polling after unmount", () => {
    const { ui, desktop, wrapper } = setup();
    ui.view = "desktop";
    vi.advanceTimersByTime(DESKTOP_POLL_MS);
    expect(desktop.reload).toHaveBeenCalledTimes(1);

    wrapper.unmount();
    vi.advanceTimersByTime(DESKTOP_POLL_MS * 3);
    expect(desktop.reload).toHaveBeenCalledTimes(1);
  });

  it("skips polling while the cli view is active", () => {
    const { ui, desktop, tick } = setup();
    ui.view = "cli";
    tick();
    expect(desktop.reload).not.toHaveBeenCalled();
  });
});
