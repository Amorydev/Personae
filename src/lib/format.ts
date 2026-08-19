import { nowSecs } from "./tauri";
import type { CliProfile, DesktopProfile } from "./types";

export const PALETTE = [
  "C2714F", "3B82F6", "22C55E", "8B5CF6", "EC4899",
  "14B8A6", "F59E0B", "EF4444", "6366F1", "06B6D4", "F97316",
];

export function prettySize(s: string | null | undefined): string {
  if (!s || s === "—") return "—";
  const m = String(s).trim().match(/^([\d.]+)\s*([KMGT]?)i?B?$/i);
  if (!m) return s;
  const unit = ({ "": "B", K: "KB", M: "MB", G: "GB", T: "TB" } as Record<string, string>)[m[2].toUpperCase()] || "B";
  return `${m[1]} ${unit}`;
}

export function relTime(secs: number | null | undefined): string {
  if (!secs) return "—";
  const d = nowSecs() - secs;
  if (d < 45) return "Just now";
  if (d < 3600) return `${Math.floor(d / 60)}m ago`;
  if (d < 86400) return `${Math.floor(d / 3600)}h ago`;
  const days = Math.floor(d / 86400);
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days}d ago`;
  if (days < 30) return `${Math.floor(days / 7)}w ago`;
  return new Date(secs * 1000).toLocaleDateString();
}

export function createdLabel(secs: number | null | undefined): string {
  if (!secs) return "—";
  const days = Math.floor((nowSecs() - secs) / 86400);
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 30) return `${days}d ago`;
  return new Date(secs * 1000).toLocaleDateString();
}

export function escapeHtml(s: unknown): string {
  return String(s).replace(/[&<>"]/g, (c) => (({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" } as Record<string, string>)[c]));
}

export interface SessionExpiry {
  label: string; // "Signed in for 42 more days" / "Signed in until 3/1/2027" / "Session expired"
  warn: boolean; // true when expired or within the warning window
}

const EXPIRY_WARNING_DAYS = 14;

// `refreshExpiresAtMs` — the refresh token's expiry, i.e. the point a real
// re-login becomes unavoidable (the short-lived access token itself
// auto-refreshes silently and isn't user-actionable, so it's not surfaced here).
export function sessionExpiry(refreshExpiresAtMs: number | null | undefined): SessionExpiry | null {
  if (!refreshExpiresAtMs) return null;
  const msLeft = refreshExpiresAtMs - Date.now();
  const daysLeft = Math.floor(msLeft / 86400000);
  if (msLeft <= 0) return { label: "Session expired — sign in again", warn: true };
  if (daysLeft < 1) return { label: "Session expires today", warn: true };
  if (daysLeft === 1) return { label: "Session expires tomorrow", warn: true };
  const warn = daysLeft <= EXPIRY_WARNING_DAYS;
  if (warn) return { label: `Session expires in ${daysLeft} days`, warn: true };
  return { label: `Signed in until ${new Date(refreshExpiresAtMs).toLocaleDateString()}`, warn: false };
}

// Live usage percentages (Claude Code's own `/usage` cache, refreshed by the
// CLI itself — see `cli::extract_usage_utilization`'s doc comment for the
// on-disk shape). "—" when neither percentage has ever been populated (e.g.
// an account that's never made a real request) — no plan-tier fallback here:
// that's a static label, not a usage number, and showing it in this slot
// reads as a live number when it isn't one.
export function usageLabel(sessionPct: number | null | undefined, weeklyPct: number | null | undefined): string {
  const parts: string[] = [];
  if (sessionPct != null) parts.push(`${sessionPct}% session`);
  if (weeklyPct != null) parts.push(`${weeklyPct}% week`);
  return parts.length ? parts.join(" · ") : "—";
}

export function cliColor(p: CliProfile | DesktopProfile | null | undefined): string {
  const key = (p as CliProfile)?.slug || p?.name || "cli";
  let hash = 2166136261;
  for (const ch of key) {
    hash ^= ch.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return `#${PALETTE[(hash >>> 0) % PALETTE.length]}`;
}

export function accentInk(hex: string | null | undefined): string {
  const h = (hex || "").replace(/^#/, "");
  if (!/^[0-9a-f]{6}$/i.test(h)) return "#000";
  const channel = (i: number) => {
    const c = parseInt(h.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  const l = 0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4);
  return (l + 0.05) / 0.05 >= 1.05 / (l + 0.05) ? "#000" : "#fff";
}
