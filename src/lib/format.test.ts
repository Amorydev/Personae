import { describe, expect, it } from "vitest";
import { PALETTE, accentInk, createdLabel, prettySize, relTime } from "./format";

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

describe("accentInk", () => {
  const luminance = (hex: string) => {
    const h = hex.replace(/^#/, "");
    const channel = (i: number) => {
      const c = parseInt(h.slice(i, i + 2), 16) / 255;
      return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4);
  };
  const contrast = (a: string, b: string) => {
    const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
    return (hi + 0.05) / (lo + 0.05);
  };

  it("clears WCAG AA against every palette tint", () => {
    for (const tint of PALETTE) {
      const ink = accentInk(`#${tint}`) === "#000" ? "#000000" : "#ffffff";
      expect(contrast(`#${tint}`, ink)).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("picks white on a dark tint and black on a light one", () => {
    expect(accentInk("#1a1a1f")).toBe("#fff");
    expect(accentInk("#F59E0B")).toBe("#000");
  });

  it("falls back to black for a malformed or missing colour", () => {
    expect(accentInk(null)).toBe("#000");
    expect(accentInk("not-a-colour")).toBe("#000");
    expect(accentInk("#abc")).toBe("#000");
  });
});
