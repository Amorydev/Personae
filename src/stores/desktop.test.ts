import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it } from "vitest";
import { useDesktopStore } from "./desktop";
import type { DesktopProfile } from "../lib/types";

function makeProfile(overrides: Partial<DesktopProfile>): DesktopProfile {
  return {
    name: "Test",
    slug: "test",
    tint: "#123456",
    running: false,
    data_size: "0B",
    data_path: "/tmp/test",
    created: 0,
    last_active: null,
    ...overrides,
  };
}

describe("desktop store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("filters profiles by name", () => {
    const store = useDesktopStore();
    store.profiles = [makeProfile({ name: "Work", slug: "work" }), makeProfile({ name: "Personal", slug: "personal" })];
    store.query = "wor";
    expect(store.filtered.map((p) => p.slug)).toEqual(["work"]);
  });

  it("current resolves the selected profile", () => {
    const store = useDesktopStore();
    store.profiles = [makeProfile({ name: "Work", slug: "work" }), makeProfile({ name: "Personal", slug: "personal" })];
    store.selected = "personal";
    expect(store.current?.name).toBe("Personal");
  });

  it("falls back to the first visible item when the selection is filtered out", async () => {
    const store = useDesktopStore();
    store.profiles = [makeProfile({ name: "Work", slug: "work" }), makeProfile({ name: "Personal", slug: "personal" })];
    store.selected = "personal";
    store.query = "wor";
    await nextTick();
    expect(store.selected).toBe("work");
  });
});
