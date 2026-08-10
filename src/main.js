// ---------- Tauri bridge (with a mock fallback for browser preview) ----------
const hasTauri = !!(window.__TAURI__ && window.__TAURI__.core);

const PALETTE = [
  "C2714F", "3B82F6", "22C55E", "8B5CF6", "EC4899",
  "14B8A6", "F59E0B", "EF4444", "6366F1", "06B6D4", "F97316",
];

const now = () => Math.floor(Date.now() / 1000);
const MOCK = [
  { name: "Personal", slug: "personal", tint: "#C2714F", running: false,
    data_size: "7.1M", data_path: "/Users/you/Library/Application Support/Claude-Personal",
    created: now() - 3600, last_active: now() - 12 },
  { name: "Work", slug: "work", tint: "#14B8A6", running: true,
    data_size: "128M", data_path: "/Users/you/Library/Application Support/Claude-Work",
    created: now() - 86400 * 9, last_active: now() - 4200 },
];

const MOCK_CLI = [
  { name: "Work CLI", slug: "work-cli", config_dir: "/Users/you/Library/Application Support/ClaudeProfilesCLI/work-cli",
    launcher_path: "/Users/you/Applications/Claude Profiles CLI/Work CLI.command",
    has_token: true, account_email: "work@corp.com", auth_kind: "subscription", data_size: "12M", created: now() - 86400, last_active: now() - 300 },
];

async function invoke(cmd, args) {
  if (hasTauri) return window.__TAURI__.core.invoke(cmd, args);
  // ---- browser mock ----
  switch (cmd) {
    case "claude_found": return true;
    case "list_profiles": return structuredClone(MOCK);
    case "create_profile": {
      const slug = args.name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
      MOCK.push({ name: args.name, slug, tint: args.color ? `#${args.color}` : "#8B5CF6",
        running: false, data_size: "0B",
        data_path: `/Users/you/Library/Application Support/Claude-${args.name}`,
        created: now(), last_active: now() });
      return;
    }
    case "set_profile_color": { const p = MOCK.find(m => m.name === args.name); if (p) p.tint = `#${args.color}`; return; }
    case "launch_profile": { const p = MOCK.find(m => m.name === args.name); if (p) p.running = true; return; }
    case "quit_profile": { const p = MOCK.find(m => m.name === args.name); if (p) p.running = false; return; }
    case "delete_profile": { const i = MOCK.findIndex(m => m.name === args.name); if (i >= 0) MOCK.splice(i, 1); return; }
    case "repair_profiles": return MOCK.length;
    case "reveal_path": console.log("reveal", args.path); return;
    case "open_url": console.log("open", args.url); return;
    case "cli_available": return true;
    case "list_cli_profiles": return structuredClone(MOCK_CLI);
    case "create_cli_profile": {
      const slug = args.name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
      MOCK_CLI.push({ name: args.name, slug, config_dir: `/Users/you/Library/Application Support/ClaudeProfilesCLI/${slug}`,
        launcher_path: `/Users/you/Applications/Claude Profiles CLI/${args.name}.command`,
        has_token: false, account_email: null, auth_kind: null, data_size: "0B", created: now(), last_active: now() });
      return;
    }
    case "set_cli_token": { const p = MOCK_CLI.find(m => m.name === args.name); if (p) { p.has_token = true; const api = String(args.token).startsWith("sk-ant-api"); p.auth_kind = api ? "console" : "subscription"; p.account_email = api ? null : "you@example.com"; } return; }
    case "open_cli_setup_token": console.log("setup-token", args.name); return;
    case "capture_cli_token": return null;
    case "launch_cli_profile": console.log("launch cli", args.name); return;
    case "delete_cli_profile": { const i = MOCK_CLI.findIndex(m => m.name === args.name); if (i >= 0) MOCK_CLI.splice(i, 1); return; }
    default: return;
  }
}

// ---------- helpers ----------
const $ = (s) => document.querySelector(s);

function burstSVG() {
  let b = "";
  const n = 12;
  for (let i = 0; i < n; i++) {
    const a = (360 / n) * i;
    b += `<rect x="46" y="7" width="8" height="34" rx="4" transform="rotate(${a} 50 50)"/>`;
  }
  return `<svg class="burst" viewBox="0 0 100 100" aria-hidden="true"><g fill="currentColor">${b}</g></svg>`;
}

function tile(p, size) {
  const el = document.createElement("div");
  el.className = `tile ${size}`;
  el.style.setProperty("--tint", p.tint || "#8a8a8a");
  el.innerHTML = burstSVG();
  if (p.running) {
    const d = document.createElement("span");
    d.className = "rundot";
    el.appendChild(d);
  }
  return el;
}

function prettySize(s) {
  if (!s || s === "—") return "—";
  const m = String(s).trim().match(/^([\d.]+)\s*([KMGT]?)i?B?$/i);
  if (!m) return s;
  const unit = { "": "B", K: "KB", M: "MB", G: "GB", T: "TB" }[m[2].toUpperCase()] || "B";
  return `${m[1]} ${unit}`;
}

function relTime(secs) {
  if (!secs) return "—";
  const d = now() - secs;
  if (d < 45) return "Just now";
  if (d < 3600) return `${Math.floor(d / 60)}m ago`;
  if (d < 86400) return `${Math.floor(d / 3600)}h ago`;
  const days = Math.floor(d / 86400);
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days}d ago`;
  if (days < 30) return `${Math.floor(days / 7)}w ago`;
  return new Date(secs * 1000).toLocaleDateString();
}

function createdLabel(secs) {
  if (!secs) return "—";
  const days = Math.floor((now() - secs) / 86400);
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 30) return `${days}d ago`;
  return new Date(secs * 1000).toLocaleDateString();
}

// ---------- state ----------
let profiles = [];
let selected = null; // slug
let query = "";
let claudeFound = true;

let view = "desktop";        // "desktop" | "cli"
let cliProfiles = [];
let cliSelected = null;      // slug
let cliAvailable = true;

function filtered() {
  const q = query.trim().toLowerCase();
  return q ? profiles.filter((p) => p.name.toLowerCase().includes(q)) : profiles;
}
function current() {
  return profiles.find((p) => p.slug === selected) || null;
}

// ---------- render: sidebar ----------
function renderList() {
  const ul = $("#list");
  ul.innerHTML = "";
  const items = filtered();
  if (!items.some((p) => p.slug === selected)) selected = items[0]?.slug ?? null;
  for (const p of items) {
    const li = document.createElement("li");
    li.className = "p-item" + (p.slug === selected ? " sel" : "");
    li.appendChild(tile(p, "sm"));
    const nm = document.createElement("span");
    nm.className = "nm";
    nm.textContent = p.name;
    li.appendChild(nm);
    li.onclick = () => { selected = p.slug; renderAll(); };
    ul.appendChild(li);
  }
}

// ---------- render: detail ----------
function renderDetail() {
  const body = $("#detail-body");
  const bar = $("#actionbar");
  const p = current();

  if (!p) {
    bar.classList.add("hidden");
    body.innerHTML = `<div class="empty"><div class="big">No profile selected</div>
      <div class="small">Create one — each gets its own login, history, and icon.</div></div>`;
    return;
  }
  bar.classList.remove("hidden");
  $("#app-desktop .detail").style.setProperty("--accent", p.tint || "#c2714f");

  body.innerHTML = "";
  const hero = document.createElement("div");
  hero.className = "hero";
  hero.appendChild(tile(p, "lg"));
  const meta = document.createElement("div");
  meta.className = "meta";
  meta.innerHTML = `<div class="name">${escapeHtml(p.name)}</div>
    <div class="sub">${p.running ? '<span class="run">Running</span>' : "Not running"}</div>`;
  hero.appendChild(meta);
  body.appendChild(hero);

  const stats = document.createElement("div");
  stats.className = "stats";
  stats.innerHTML = `
    <div class="stat"><div class="k">Storage</div><div class="v">${prettySize(p.data_size)}</div></div>
    <div class="stat"><div class="k">Last active</div><div class="v">${relTime(p.last_active)}</div></div>
    <div class="stat"><div class="k">Created</div><div class="v">${createdLabel(p.created)}</div></div>`;
  body.appendChild(stats);

  const field = document.createElement("div");
  field.className = "field";
  field.innerHTML = `<div class="k">Data directory</div>`;
  const path = document.createElement("div");
  path.className = "path";
  path.textContent = p.data_path;
  path.title = "Open in Finder";
  path.onclick = () => invoke("reveal_path", { path: p.data_path });
  field.appendChild(path);
  body.appendChild(field);

  // action bar
  const primary = $("#primary");
  primary.innerHTML = p.running
    ? `Quit ${escapeHtml(p.name)} <kbd>⏎</kbd>`
    : `Launch ${escapeHtml(p.name)} <kbd>⏎</kbd>`;
  primary.onclick = () => (p.running ? doQuit(p) : doLaunch(p));
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
}

function renderFooter() {
  const el = $("#foot-status");
  if (claudeFound) { el.className = "ok"; el.style.color = ""; el.textContent = "● Activated"; }
  else { el.className = "link"; el.style.color = "var(--warn)"; el.textContent = "⚠ Claude Desktop not found"; }
}

function renderAll() { renderList(); renderDetail(); }

// ---------- data ----------
async function reload(keep = true) {
  try {
    claudeFound = await invoke("claude_found");
    const w = $("#warn");
    if (!claudeFound) { w.classList.remove("hidden"); w.textContent = "⚠️ Claude Desktop not found. Install it, or set CLAUDE_APP."; }
    else w.classList.add("hidden");
    profiles = await invoke("list_profiles");
    if (!keep || !profiles.some((p) => p.slug === selected)) selected = profiles[0]?.slug ?? null;
    renderAll();
    renderFooter();
  } catch (e) { console.error(e); }
}

// ---------- actions ----------
async function doLaunch(p) { try { await invoke("launch_profile", { name: p.name }); } catch (e) {} setTimeout(() => reload(), 1200); }
async function doQuit(p) { try { await invoke("quit_profile", { name: p.name }); } catch (e) {} setTimeout(() => reload(), 600); }

// create modal
function openCreate() {
  $("#new-name").value = "";
  buildSwatches($("#swatches"), PALETTE[0]);
  $("#create-modal").classList.remove("hidden");
  setTimeout(() => $("#new-name").focus(), 30);
}
async function doCreate() {
  const name = $("#new-name").value.trim();
  if (!name) return;
  const color = selectedSwatch($("#swatches"));
  try { await invoke("create_profile", { name, color, isolation: "env" }); }
  catch (e) { alert(String(e)); return; }
  $("#create-modal").classList.add("hidden");
  await reload(false);
  const made = profiles.find((p) => p.name === name);
  if (made) { selected = made.slug; renderAll(); }
}

// edit (accent) modal
function openEdit() {
  const p = current(); if (!p) return;
  buildSwatches($("#edit-swatches"), (p.tint || "").replace("#", ""), async (hex) => {
    try { await invoke("set_profile_color", { name: p.name, color: hex }); } catch (e) {}
    $("#edit-modal").classList.add("hidden");
    await reload();
  });
  $("#edit-modal").classList.remove("hidden");
}

// delete modal
let pendingDelete = null;
function openDelete() {
  const p = current(); if (!p) return;
  pendingDelete = p;
  $("#del-title").textContent = `Delete “${p.name}”?`;
  $("#purge").checked = false;
  $("#del-modal").classList.remove("hidden");
}
async function doDelete() {
  if (!pendingDelete) return;
  const p = pendingDelete, purge = $("#purge").checked;
  $("#del-modal").classList.add("hidden");
  pendingDelete = null;
  try { await invoke("delete_profile", { name: p.name, purge }); } catch (e) {}
  await reload(false);
}

// swatch builder
function buildSwatches(host, selHex, onPick) {
  host.innerHTML = "";
  const sel = (selHex || PALETTE[0]).toUpperCase();
  host.dataset.pick = sel;
  for (const hex of PALETTE) {
    const b = document.createElement("div");
    b.className = "sw" + (hex.toUpperCase() === sel ? " sel" : "");
    b.style.background = `#${hex}`;
    b.onclick = () => {
      host.dataset.pick = hex;
      [...host.children].forEach((c) => c.classList.remove("sel"));
      b.classList.add("sel");
      if (onPick) onPick(hex);
    };
    host.appendChild(b);
  }
}
function selectedSwatch(host) { return host.dataset.pick || PALETTE[0]; }

function closeModals() {
  $("#create-modal").classList.add("hidden");
  $("#edit-modal").classList.add("hidden");
  $("#del-modal").classList.add("hidden");
  $("#token-modal")?.classList.add("hidden");
  $("#cli-create-modal")?.classList.add("hidden");
  $("#cli-del-modal")?.classList.add("hidden");
  if (typeof stopCapturePolling === "function") stopCapturePolling();
  if (typeof setTokenStatus === "function") setTokenStatus("");
  pendingDelete = null;
  tokenTarget = null;
}
function anyModalOpen() {
  return ![...document.querySelectorAll(".modal")].every((m) => m.classList.contains("hidden"));
}

// ---------- keyboard ----------
function moveSelection(delta) {
  const items = filtered();
  if (!items.length) return;
  let i = items.findIndex((p) => p.slug === selected);
  i = Math.max(0, Math.min(items.length - 1, (i < 0 ? 0 : i) + delta));
  selected = items[i].slug;
  renderAll();
}

document.addEventListener("keydown", (e) => {
  const meta = e.metaKey || e.ctrlKey;
  const typing = ["INPUT", "TEXTAREA"].includes(document.activeElement?.tagName);

  if (e.key === "Escape") { if (anyModalOpen()) closeModals(); return; }
  if (anyModalOpen()) {
    if (e.key === "Enter" && !$("#create-modal").classList.contains("hidden")) { e.preventDefault(); doCreate(); }
    if (e.key === "Enter" && !$("#cli-create-modal").classList.contains("hidden")) { e.preventDefault(); doCliCreate(); }
    return;
  }
  if (view !== "desktop") return;   // CLI view uses buttons; don't drive the hidden Desktop model
  if (meta && e.key.toLowerCase() === "n") { e.preventDefault(); openCreate(); return; }
  if (meta && e.key.toLowerCase() === "l") { e.preventDefault(); const p = current(); if (p && !p.running) doLaunch(p); return; }
  if (meta && e.key.toLowerCase() === "r") { e.preventDefault(); invoke("repair_profiles").then(() => reload()); return; }
  if (e.key === "/" && !typing) { e.preventDefault(); $("#search").focus(); return; }
  if (typing) return;
  if (e.key === "Enter") { const p = current(); if (p) (p.running ? doQuit(p) : doLaunch(p)); }
  else if (e.key === "ArrowDown") { e.preventDefault(); moveSelection(1); }
  else if (e.key === "ArrowUp") { e.preventDefault(); moveSelection(-1); }
  else if (e.key === "Backspace" || e.key === "Delete") { e.preventDefault(); openDelete(); }
});

// ---------- CLI view ----------
function cliCurrent() { return cliProfiles.find((p) => p.slug === cliSelected) || null; }

function setView(v) {
  view = v;
  document.querySelectorAll(".segmented .seg").forEach((b) => b.classList.toggle("sel", b.dataset.view === v));
  // Desktop content root is the whole `.app` grid (#app-desktop: sidebar + detail), not just <main> —
  // hiding only <main> would leave the Desktop sidebar showing next to the CLI view's own sidebar.
  $("#app-desktop").classList.toggle("hidden", v === "cli");
  $("#cli-view").classList.toggle("hidden", v !== "cli");
  if (v === "cli") reloadCli();
}

async function reloadCli() {
  try {
    cliAvailable = await invoke("cli_available");
    $("#cli-new-side").disabled = !cliAvailable;
    cliProfiles = await invoke("list_cli_profiles");
    if (!cliProfiles.some((p) => p.slug === cliSelected)) cliSelected = cliProfiles[0]?.slug ?? null;
    renderCliList();
    renderCliDetail();
  } catch (e) { console.error(e); }
}

function renderCliList() {
  const ul = $("#cli-list");
  ul.innerHTML = "";
  for (const p of cliProfiles) {
    const li = document.createElement("li");
    li.className = "p-item" + (p.slug === cliSelected ? " sel" : "");
    const nm = document.createElement("span");
    nm.className = "nm";
    nm.textContent = p.name;
    li.appendChild(nm);
    if (!p.has_token) {
      const w = document.createElement("span");
      w.className = "badge warn";
      w.textContent = "no credential";
      li.appendChild(w);
    }
    li.onclick = () => { cliSelected = p.slug; renderCliList(); renderCliDetail(); };
    ul.appendChild(li);
  }
}

function renderCliDetail() {
  const body = $("#cli-detail-body");
  const bar = $("#cli-actionbar");
  if (!cliAvailable) {
    bar.classList.add("hidden");
    body.innerHTML = `<div class="empty"><div class="big">CLI profiles need Claude Code</div>
      <div class="small">Install the <code>claude</code> CLI (macOS only for now).</div></div>`;
    return;
  }
  const p = cliCurrent();
  if (!p) {
    bar.classList.add("hidden");
    body.innerHTML = `<div class="empty"><div class="big">No CLI account</div>
      <div class="small">Create one, then set its credential — a subscription token (<code>claude setup-token</code>) or a Console API key.</div></div>`;
    return;
  }
  bar.classList.remove("hidden");
  const kind = p.auth_kind === "console" ? "Console API" : p.auth_kind === "subscription" ? "Subscription" : "";
  const sub = p.account_email ? escapeHtml(p.account_email)
    : (p.has_token ? (kind ? `${kind} credential set` : "Credential set")
                   : '<span style="color:var(--warn)">No credential — set one to use this account</span>');
  body.innerHTML = `
    <div class="hero"><div class="meta">
      <div class="name">${escapeHtml(p.name)}</div>
      <div class="sub">${sub}</div>
    </div></div>
    <div class="stats">
      <div class="stat"><div class="k">Auth</div><div class="v">${p.has_token ? (kind || "✓ set") : "— none"}</div></div>
      <div class="stat"><div class="k">Storage</div><div class="v">${prettySize(p.data_size)}</div></div>
      <div class="stat"><div class="k">Last active</div><div class="v">${relTime(p.last_active)}</div></div>
    </div>`;
  const field = document.createElement("div");
  field.className = "field";
  field.innerHTML = `<div class="k">Config directory</div>`;
  const path = document.createElement("div");
  path.className = "path";
  path.textContent = p.config_dir;
  path.title = "Open in Finder";
  path.onclick = () => invoke("reveal_path", { path: p.config_dir });
  field.appendChild(path);
  body.appendChild(field);

  const primary = $("#cli-primary");
  primary.textContent = `Launch ${p.name}`;
  primary.disabled = !p.has_token;
  primary.onclick = () => doCliLaunch(p);
}

async function doCliLaunch(p) {
  try { await invoke("launch_cli_profile", { name: p.name }); }
  catch (e) { alert(String(e)); }
}

function openCliCreate() {
  $("#cli-new-name").value = "";
  $("#cli-create-modal").classList.remove("hidden");
  setTimeout(() => $("#cli-new-name").focus(), 30);
}
async function doCliCreate() {
  const name = $("#cli-new-name").value.trim();
  if (!name) return;
  try { await invoke("create_cli_profile", { name }); }
  catch (e) { alert(String(e)); return; }
  $("#cli-create-modal").classList.add("hidden");
  await reloadCli();
  const made = cliProfiles.find((p) => p.name === name);
  if (made) { cliSelected = made.slug; renderCliList(); renderCliDetail(); openTokenModal(); }
}

let tokenTarget = null;
let capturePoll = null;
function setTokenStatus(msg) { const s = $("#token-status"); if (s) s.textContent = msg || ""; }
function stopCapturePolling() { if (capturePoll) { clearInterval(capturePoll); capturePoll = null; } }

function openTokenModal() {
  const p = cliCurrent(); if (!p) return;
  tokenTarget = p;
  stopCapturePolling();
  setTokenStatus("");
  $("#token-input").value = "";
  $("#token-modal").classList.remove("hidden");
  setTimeout(() => $("#token-input").focus(), 30);
}
function closeTokenModal() {
  stopCapturePolling();
  setTokenStatus("");
  $("#token-modal").classList.add("hidden");
  tokenTarget = null;
}

// Subscription path: after opening `setup-token` in Terminal, poll for the token
// the app captures automatically — no copy-paste needed.
function startCapturePolling(name) {
  stopCapturePolling();
  setTokenStatus("⏳ Waiting — finish the login in the Terminal window…");
  let tries = 0;
  capturePoll = setInterval(async () => {
    if (++tries > 150) { stopCapturePolling(); setTokenStatus("Timed out. Paste the token below, or try again."); return; }
    let kind;
    try { kind = await invoke("capture_cli_token", { name }); } catch (e) { return; }
    if (kind) {
      const label = kind === "console" ? "Console API" : "Subscription";
      stopCapturePolling();
      setTokenStatus(`✅ ${label} credential captured.`);
      await reloadCli();
      setTimeout(closeTokenModal, 700);
    }
  }, 2000);
}

async function doTokenSave() {
  if (!tokenTarget) return;
  const token = $("#token-input").value.trim();
  if (!token) return;
  try { await invoke("set_cli_token", { name: tokenTarget.name, token }); }
  catch (e) { alert(String(e)); return; }
  closeTokenModal();
  await reloadCli();
}

function openCliDelete() {
  const p = cliCurrent(); if (!p) return;
  $("#cli-del-title").textContent = `Delete “${p.name}”?`;
  $("#cli-del-modal").classList.remove("hidden");
}
async function doCliDelete() {
  const p = cliCurrent(); if (!p) return;
  $("#cli-del-modal").classList.add("hidden");
  try { await invoke("delete_cli_profile", { name: p.name, purge: true }); }
  catch (e) { alert(String(e)); return; }
  await reloadCli();
}

// ---------- wire up ----------
window.addEventListener("DOMContentLoaded", () => {
  $("#search").addEventListener("input", (e) => { query = e.target.value; renderList(); });
  $("#new-side").onclick = openCreate;
  $("#new-top").onclick = openCreate;
  $("#edit").onclick = openEdit;
  $("#create-cancel").onclick = closeModals;
  $("#create-go").onclick = doCreate;
  $("#edit-cancel").onclick = closeModals;
  $("#del-cancel").onclick = closeModals;
  $("#del-go").onclick = doDelete;
  $("#repair-link").onclick = () => invoke("repair_profiles").then(() => reload());
  $("#feedback").onclick = () => invoke("open_url", { url: "mailto:amory.dev@gmail.com?subject=Personae%20feedback" });
  document.querySelectorAll(".segmented .seg").forEach((b) => b.onclick = () => setView(b.dataset.view));
  $("#cli-new-side").onclick = openCliCreate;
  $("#cli-create-cancel").onclick = closeModals;
  $("#cli-create-go").onclick = doCliCreate;
  $("#cli-settoken").onclick = openTokenModal;
  $("#cli-delete").onclick = openCliDelete;
  $("#cli-del-cancel").onclick = closeModals;
  $("#cli-del-go").onclick = doCliDelete;
  $("#token-open").onclick = async () => {
    const p = cliCurrent(); if (!p) return;
    try { await invoke("open_cli_setup_token", { name: p.name }); }
    catch (e) { alert(String(e)); return; }
    startCapturePolling(p.name);
  };
  $("#token-console").onclick = () => invoke("open_url", { url: "https://console.anthropic.com/settings/keys" });
  $("#token-cancel").onclick = closeTokenModal;
  $("#token-save").onclick = doTokenSave;
  // #token-modal already gets outside-click-to-close from the generic ".modal" loop below.
  for (const m of document.querySelectorAll(".modal")) {
    m.addEventListener("click", (e) => { if (e.target === m) closeModals(); });
  }
  reload();
});
