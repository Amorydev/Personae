import { describe, expect, it } from "vitest";
import { createdLabel, prettySize, relTime } from "./format";

describe("prettySize", () => {
  it("passes through the placeholder", () => {
    expect(prettySize("—")).toBe("—");
    expect(prettySize(null)).toBe("—");
  });

  it("expands unit abbreviations", () => {
    expect(prettySize("7.1M")).toBe("7.1 MB");
    expect(prettySize("128G")).toBe("128 GB");
    expect(prettySize("0B")).toBe("0 B");
  });

  it("returns the raw string when it doesn't match the pattern", () => {
    expect(prettySize("n/a")).toBe("n/a");
  });
});

describe("relTime", () => {
  it("returns the placeholder for falsy input", () => {
    expect(relTime(null)).toBe("—");
    expect(relTime(0)).toBe("—");
  });

  it("labels very recent timestamps as just now", () => {
    const now = Math.floor(Date.now() / 1000);
    expect(relTime(now - 5)).toBe("Just now");
  });

  it("labels yesterday distinctly from N days ago", () => {
    const now = Math.floor(Date.now() / 1000);
    expect(relTime(now - 86400)).toBe("Yesterday");
    expect(relTime(now - 86400 * 3)).toBe("3d ago");
  });
});

describe("createdLabel", () => {
  it("labels today and yesterday", () => {
    const now = Math.floor(Date.now() / 1000);
    expect(createdLabel(now)).toBe("Today");
    expect(createdLabel(now - 86400)).toBe("Yesterday");
  });
});
