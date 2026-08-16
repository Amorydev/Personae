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
    logged_in: true, account_email: "work@corp.com", data_size: "12M", created: now() - 86400, last_active: now() - 300 },
  { name: "Personal CLI", slug: "personal-cli", config_dir: "/Users/you/Library/Application Support/ClaudeProfilesCLI/personal-cli",
    launcher_path: "/Users/you/Applications/Claude Profiles CLI/Personal CLI.command",
    logged_in: false, account_email: null, data_size: "0B", created: now() - 3600, last_active: null },
];

const MOCK_WS = [
  { id: "cursorwork-cli/Users/you/Projects/demo", account_slug: "work-cli", account_name: "Work CLI",
    ide_id: "cursor", ide_name: "Cursor", project_path: "/Users/you/Projects/demo",
    created: now()-3600, last_opened: now()-600 },
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
        logged_in: false, account_email: null, data_size: "0B", created: now(), last_active: now() });
      return;
    }
    case "login_cli_profile": console.log("login", args.name); return;
    case "launch_cli_profile": console.log("launch cli", args.name); return;
    case "delete_cli_profile": { const i = MOCK_CLI.findIndex(m => m.name === args.name); if (i >= 0) MOCK_CLI.splice(i, 1); return; }
    case "list_ides": return [
      { id: "vscode", name: "Visual Studio Code", cli_path: "/usr/local/bin/code" },
      { id: "cursor", name: "Cursor", cli_path: "/Applications/Cursor.app/…/cursor" },
    ];
    case "pick_folder": return "/Users/you/Projects/demo";
    case "open_in_ide": console.log("open_in_ide", args); return;
    case "list_workspaces": return structuredClone(MOCK_WS);
    case "save_workspace": { const id = `${args.ideId}${args.accountSlug}${args.projectPath}`;
      if (!MOCK_WS.some(w=>w.id===id)) MOCK_WS.push({ id, account_slug: args.accountSlug, account_name: args.accountName,
        ide_id: args.ideId, ide_name: args.ideName, project_path: args.projectPath,
        created: args.now, last_opened: args.now }); return; }
    case "delete_workspace": { const i = MOCK_WS.findIndex(w=>w.id===args.id); if(i>=0) MOCK_WS.splice(i,1); return; }
    case "open_workspace": console.log("open_workspace", args.id); return;
    default: return;
  }
}

// ---------- helpers ----------
const $ = (s) => document.querySelector(s);
let toastTimer = null;

function showToast(message, tone = "info") {
  let toast = $("#app-toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.id = "app-toast";
    toast.setAttribute("role", "status");
    toast.setAttribute("aria-live", "polite");
    document.body.appendChild(toast);
  }
  toast.className = `app-toast ${tone} visible`;
  toast.textContent = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.remove("visible"), 4200);
}

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
let cliQuery = "";
let cliAvailable = true;
let cliWorkspaces = [];      // all saved workspaces (rendered nested under their account)

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
  ul.setAttribute("role", "listbox");
  ul.setAttribute("aria-label", "Desktop profiles");
  $("#profile-count").textContent = String(profiles.length);
  if (!items.some((p) => p.slug === selected)) selected = items[0]?.slug ?? null;
  if (!items.length) {
    const empty = document.createElement("li");
    empty.className = "cli-list-empty";
    empty.textContent = "No matching profiles";
    ul.appendChild(empty);
    return;
  }
  for (const p of items) {
    const li = document.createElement("li");
    const isSelected = p.slug === selected;
    li.className = "p-item desktop-profile-item" + (isSelected ? " sel" : "");
    li.setAttribute("role", "option");
    li.setAttribute("aria-selected", String(isSelected));
    li.tabIndex = isSelected ? 0 : -1;
    li.style.setProperty("--profile-color", p.tint || "#c2714f");
    li.appendChild(tile(p, "account"));
    const copy = document.createElement("span");
    copy.className = "desktop-profile-copy";
    const nm = document.createElement("span");
    nm.className = "nm";
    nm.textContent = p.name;
    const activity = document.createElement("span");
    activity.className = "desktop-profile-activity" + (p.running ? " running" : "");
    activity.textContent = p.running ? "Running" : `Last active ${relTime(p.last_active)}`;
    copy.append(nm, activity);
    const dot = document.createElement("span");
    dot.className = `account-status-dot ${p.running ? "is-ok" : "is-idle"}`;
    dot.setAttribute("aria-hidden", "true");
    li.append(copy, dot);
    const choose = () => { selected = p.slug; renderAll(); };
    li.onclick = choose;
    li.onkeydown = (e) => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); choose(); }
    };
    ul.appendChild(li);
  }
}

// ---------- render: detail ----------
function renderDetail() {
  const body = $("#detail-body");
  const p = current();

  if (!p) {
    body.innerHTML = `<div class="empty cli-empty"><div class="empty-mark">✦</div>
      <div class="big">Create your first Desktop profile</div>
      <div class="small">Each profile keeps its own Claude login, history, data, and appearance.</div>
      <button id="desktop-empty-create" class="primary">Create profile</button></div>`;
    $("#desktop-empty-create").onclick = openCreate;
    return;
  }
  const accent = p.tint || "#c2714f";
  $("#app-desktop").style.setProperty("--accent", accent);
  const status = p.running
    ? '<span class="status-pill running"><span></span>Running</span>'
    : '<span class="status-pill ready"><span></span>Ready</span>';
  const subtitle = p.running ? "Claude Desktop is active in this profile" : `Last active ${relTime(p.last_active)}`;
  body.innerHTML = `
    <div class="desktop-content">
      <section class="cli-profile-hero">
        <div class="cli-hero-identity">
          <span id="desktop-mark-slot"></span>
          <div class="cli-hero-copy">
            <div class="cli-title-line"><h1>${escapeHtml(p.name)}</h1>${status}</div>
            <p>${subtitle}</p>
          </div>
        </div>
        <div class="cli-hero-actions">
          <button id="desktop-primary" class="primary"><span aria-hidden="true">${p.running ? "■" : "↗"}</span> ${p.running ? "Quit Desktop" : "Launch Desktop"}</button>
          <button id="desktop-reveal" class="btn">Show data…</button>
        </div>
      </section>
      <section class="cli-metrics" aria-label="Profile overview">
        <div><span>Storage</span><strong>${prettySize(p.data_size)}</strong></div>
        <div><span>Last active</span><strong>${relTime(p.last_active)}</strong></div>
        <div><span>Created</span><strong>${createdLabel(p.created)}</strong></div>
      </section>
      <section class="cli-section">
        <div class="section-heading"><div><h2>Profile data</h2>
          <p>Login, history, cache, and settings isolated for this profile.</p></div></div>
        <div class="profile-data-card">
          <span class="profile-data-icon" aria-hidden="true">◇</span>
          <span class="profile-data-copy"><strong>Claude Desktop data</strong><code>${escapeHtml(p.data_path)}</code></span>
          <button id="desktop-reveal-inline" class="btn compact">Show in Finder</button>
        </div>
      </section>
      <details class="cli-advanced desktop-advanced">
        <summary><span><strong>Advanced details</strong><small>Appearance, repair, and profile actions</small></span><span class="chevron" aria-hidden="true">⌄</span></summary>
        <div class="advanced-body">
          <div class="desktop-accent-row">
            <span class="accent-preview" style="--preview-color:${accent}"></span>
            <div><strong>Profile accent</strong><span>Used for this profile's icon and actions.</span></div>
            <button id="desktop-edit" class="btn compact">Change…</button>
          </div>
          <div class="advanced-actions desktop-utility-actions">
            <button id="desktop-repair" class="text-btn">Repair launchers</button>
            <button id="desktop-feedback" class="text-btn">Send feedback</button>
            <button id="desktop-delete" class="danger-ghost">Delete profile…</button>
          </div>
        </div>
      </details>
    </div>`;
  $("#desktop-mark-slot").replaceWith(tile(p, "md"));
  $("#desktop-primary").onclick = () => (p.running ? doQuit(p) : doLaunch(p));
  $("#desktop-reveal").onclick = () => invoke("reveal_path", { path: p.data_path });
  $("#desktop-reveal-inline").onclick = () => invoke("reveal_path", { path: p.data_path });
  $("#desktop-edit").onclick = openEdit;
  $("#desktop-repair").onclick = () => invoke("repair_profiles").then(() => reload());
  $("#desktop-feedback").onclick = () => invoke("open_url", { url: "mailto:amory.dev@gmail.com?subject=Personae%20feedback" });
  $("#desktop-delete").onclick = openDelete;
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
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
  } catch (e) { console.error(e); }
}

// ---------- actions ----------
async function doLaunch(p) {
  try { await invoke("launch_profile", { name: p.name }); }
  catch (e) { showToast(String(e), "error"); return; }
  setTimeout(() => reload(), 1200);
}
async function doQuit(p) {
  try { await invoke("quit_profile", { name: p.name }); }
  catch (e) { showToast(String(e), "error"); return; }
  setTimeout(() => reload(), 600);
}

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
    const b = document.createElement("button");
    b.type = "button";
    b.className = "sw" + (hex.toUpperCase() === sel ? " sel" : "");
    b.setAttribute("aria-label", `Accent #${hex}`);
    b.setAttribute("aria-pressed", String(hex.toUpperCase() === sel));
    b.title = `#${hex}`;
    b.style.background = `#${hex}`;
    b.onclick = () => {
      host.dataset.pick = hex;
      [...host.children].forEach((c) => { c.classList.remove("sel"); c.setAttribute("aria-pressed", "false"); });
      b.classList.add("sel");
      b.setAttribute("aria-pressed", "true");
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
  $("#cli-create-modal")?.classList.add("hidden");
  $("#cli-del-modal")?.classList.add("hidden");
  $("#ide-modal")?.classList.add("hidden");
  pendingDelete = null;
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
  document.querySelector('#list [aria-selected="true"]')?.focus();
}

function moveCliSelection(delta) {
  const items = filteredCli();
  if (!items.length) return;
  let i = items.findIndex((p) => p.slug === cliSelected);
  i = Math.max(0, Math.min(items.length - 1, (i < 0 ? 0 : i) + delta));
  selectCliProfile(items[i]);
  document.querySelector('#cli-list [aria-selected="true"]')?.focus();
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
  if (view === "cli") {
    const interactive = document.activeElement?.matches?.("button, select, [role=button]");
    if (meta && e.key.toLowerCase() === "n") { e.preventDefault(); openCliCreate(); return; }
    if (e.key === "/" && !typing) { e.preventDefault(); $("#cli-search").focus(); return; }
    if (typing || interactive) return;
    if (e.key === "ArrowDown") { e.preventDefault(); moveCliSelection(1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); moveCliSelection(-1); }
    else if (e.key === "Enter") {
      const p = cliCurrent();
      if (p) p.logged_in ? doCliLaunch(p) : doCliLogin();
    }
    return;
  }
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

function filteredCli() {
  const q = cliQuery.trim().toLowerCase();
  if (!q) return cliProfiles;
  return cliProfiles.filter((p) =>
    p.name.toLowerCase().includes(q) || String(p.account_email || "").toLowerCase().includes(q));
}

function cliColor(p) {
  const key = p?.slug || p?.name || "cli";
  let hash = 2166136261;
  for (const ch of key) {
    hash ^= ch.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return `#${PALETTE[(hash >>> 0) % PALETTE.length]}`;
}

function cliMark(p, size = "sm") {
  const mark = document.createElement("span");
  mark.className = `cli-mark ${size}`;
  mark.style.setProperty("--account-color", cliColor(p));
  mark.innerHTML = `<span aria-hidden="true">&gt;_</span>`;
  return mark;
}

function selectCliProfile(p) {
  cliSelected = p.slug;
  renderCliList();
  renderCliDetail();
}

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
    cliWorkspaces = await invoke("list_workspaces");
    if (!cliProfiles.some((p) => p.slug === cliSelected)) cliSelected = cliProfiles[0]?.slug ?? null;
    renderCliList();
    renderCliDetail();
  } catch (e) { console.error(e); }
}

function renderCliList() {
  const ul = $("#cli-list");
  ul.innerHTML = "";
  ul.setAttribute("role", "listbox");
  ul.setAttribute("aria-label", "CLI accounts");
  const items = filteredCli();
  $("#cli-account-count").textContent = String(cliProfiles.length);
  if (items.length && !items.some((p) => p.slug === cliSelected)) cliSelected = items[0].slug;
  if (!items.length) {
    const empty = document.createElement("li");
    empty.className = "cli-list-empty";
    empty.textContent = "No matching accounts";
    ul.appendChild(empty);
    return;
  }
  for (const p of items) {
    const li = document.createElement("li");
    const selected = p.slug === cliSelected;
    li.className = "p-item cli-account-item" + (selected ? " sel" : "");
    li.setAttribute("role", "option");
    li.setAttribute("aria-selected", String(selected));
    li.tabIndex = selected ? 0 : -1;
    li.appendChild(cliMark(p));
    const copy = document.createElement("span");
    copy.className = "cli-account-copy";
    const nm = document.createElement("span");
    nm.className = "nm";
    nm.textContent = p.name;
    const identity = document.createElement("span");
    identity.className = "cli-account-identity" + (p.logged_in ? "" : " needs-login");
    identity.textContent = p.logged_in ? (p.account_email || "Signed in") : "Needs login";
    copy.append(nm, identity);
    li.appendChild(copy);
    const status = document.createElement("span");
    status.className = `account-status-dot ${p.logged_in ? "is-ok" : "is-warn"}`;
    status.title = p.logged_in ? "Signed in" : "Needs login";
    status.setAttribute("aria-hidden", "true");
    li.appendChild(status);
    li.onclick = () => selectCliProfile(p);
    li.onkeydown = (e) => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); selectCliProfile(p); }
    };
    ul.appendChild(li);
  }
}

function renderCliDetail() {
  const body = $("#cli-detail-body");
  if (!cliAvailable) {
    body.innerHTML = `<div class="empty cli-empty"><div class="empty-mark">&gt;_</div>
      <div class="big">Claude Code is required</div>
      <div class="small">Install the <code>claude</code> CLI to create isolated accounts, then reopen Personae.</div></div>`;
    return;
  }
  const p = cliCurrent();
  if (!p) {
    body.innerHTML = `<div class="empty cli-empty"><div class="empty-mark">&gt;_</div>
      <div class="big">Create your first CLI account</div>
      <div class="small">Each account keeps its own Claude login, history, and project environments.</div>
      <button id="cli-empty-create" class="primary">Create account</button></div>`;
    $("#cli-empty-create").onclick = openCliCreate;
    return;
  }

  $("#cli-view").style.setProperty("--accent", cliColor(p));
  const status = p.logged_in
    ? '<span class="status-pill signed-in"><span></span>Signed in</span>'
    : '<span class="status-pill needs-login"><span></span>Needs login</span>';
  const identity = p.account_email
    ? escapeHtml(p.account_email)
    : p.logged_in ? "Claude account connected" : "Sign in once to activate this environment";
  const primaryActions = p.logged_in
    ? `<button id="cli-primary" class="primary"><span aria-hidden="true">↗</span> Launch CLI</button>
       <button id="cli-open-ide" class="btn">Open project…</button>`
    : `<button id="cli-login" class="primary">Log in account</button>
       <button class="btn" disabled title="Log in first">Launch CLI</button>`;
  body.innerHTML = `
    <div class="cli-content">
      <section class="cli-profile-hero">
        <div class="cli-hero-identity">
          <span class="cli-mark lg" style="--account-color:${cliColor(p)}"><span aria-hidden="true">&gt;_</span></span>
          <div class="cli-hero-copy">
            <div class="cli-title-line"><h1>${escapeHtml(p.name)}</h1>${status}</div>
            <p>${identity}</p>
          </div>
        </div>
        <div class="cli-hero-actions">${primaryActions}</div>
      </section>
      ${p.logged_in ? `
        <section class="cli-metrics" aria-label="Account overview">
          <div><span>Storage</span><strong>${prettySize(p.data_size)}</strong></div>
          <div><span>Last active</span><strong>${relTime(p.last_active)}</strong></div>
          <div><span>Created</span><strong>${createdLabel(p.created)}</strong></div>
        </section>` : `
        <section class="cli-setup-card">
          <div class="setup-icon" aria-hidden="true">↳</div>
          <div><h2>Finish setting up this account</h2>
            <p>Personae will open Claude's secure sign-in in Terminal. When it is complete, return here to launch or connect projects.</p>
          </div>
        </section>`}
      <section id="cli-workspaces-section" class="cli-section"></section>
      <details class="cli-advanced">
        <summary><span><strong>Advanced details</strong><small>Config, login, and account actions</small></span><span class="chevron" aria-hidden="true">⌄</span></summary>
        <div class="advanced-body">
          <div class="advanced-path-row">
            <div><span class="advanced-label">Config directory</span><code>${escapeHtml(p.config_dir)}</code></div>
            <button id="cli-reveal" class="btn compact">Show in Finder</button>
          </div>
          <div class="advanced-actions">
            ${p.logged_in ? '<button id="cli-relogin" class="text-btn">Switch or refresh login</button>' : ""}
            <button id="cli-delete" class="danger-ghost">Delete account…</button>
          </div>
        </div>
      </details>
    </div>`;

  const wsField = $("#cli-workspaces-section");
  wsField.innerHTML = `<div class="section-heading"><div><h2>Workspaces</h2>
    <p>Projects that reopen with this account active.</p></div></div>`;
  const wss = cliWorkspaces.filter((w) => w.account_slug === p.slug);
  if (!wss.length) {
    const empty = document.createElement("div");
    empty.className = "workspace-empty";
    empty.innerHTML = `<span class="workspace-empty-icon" aria-hidden="true">◇</span>
      <div><strong>No projects yet</strong><p>${p.logged_in ? "Open a project to bind it to this account." : "Log in before connecting a project."}</p></div>`;
    if (p.logged_in) {
      const add = document.createElement("button");
      add.className = "btn compact";
      add.textContent = "Open project…";
      add.onclick = openIdeModal;
      empty.appendChild(add);
    }
    wsField.appendChild(empty);
  } else {
    const list = document.createElement("div");
    list.className = "workspace-list";
    for (const w of wss) {
      const row = document.createElement("div");
      row.className = "workspace-row";
      const open = document.createElement("button");
      open.className = "workspace-open";
      const folder = w.project_path.replace(/\/+$/, "").split("/").pop() || w.project_path;
      const icon = document.createElement("span"); icon.className = "workspace-icon"; icon.textContent = "◇";
      const copy = document.createElement("span"); copy.className = "workspace-copy";
      const label = document.createElement("strong"); label.textContent = folder;
      const path = document.createElement("span"); path.textContent = w.project_path;
      copy.append(label, path);
      const meta = document.createElement("span"); meta.className = "workspace-meta";
      const badge = document.createElement("span"); badge.className = "ide-badge";
      badge.textContent = w.ide_name;
      const last = document.createElement("span"); last.textContent = relTime(w.last_opened);
      meta.append(badge, last);
      const del = document.createElement("button"); del.className = "workspace-remove"; del.textContent = "×";
      del.title = "Remove this workspace";
      del.setAttribute("aria-label", `Remove ${folder}`);
      del.onclick = async (e) => {
        e.stopPropagation();
        try { await invoke("delete_workspace", { id: w.id }); }
        catch (e) { showToast(String(e), "error"); }
        await reloadCli();
      };
      open.append(icon, copy, meta);
      open.title = `${w.project_path} — open in ${w.ide_name}`;
      open.setAttribute("aria-label", `Open ${folder} in ${w.ide_name}`);
      open.onclick = async () => {
        try { await invoke("open_workspace", { id: w.id, now: now() }); }
        catch (e) { showToast(String(e), "error"); }
        await reloadCli();
      };
      row.append(open, del);
      list.appendChild(row);
    }
    wsField.appendChild(list);
  }

  if (p.logged_in) {
    $("#cli-primary").onclick = () => doCliLaunch(p);
    $("#cli-open-ide").onclick = openIdeModal;
    $("#cli-relogin").onclick = doCliLogin;
  } else {
    $("#cli-login").onclick = doCliLogin;
  }
  $("#cli-reveal").onclick = () => invoke("reveal_path", { path: p.config_dir });
  $("#cli-delete").onclick = openCliDelete;
}

async function doCliLogin() {
  const p = cliCurrent(); if (!p) return;
  try { await invoke("login_cli_profile", { name: p.name }); }
  catch (e) { showToast(String(e), "error"); return; }
  showToast(`Sign-in opened in Terminal for “${p.name}”. Return here when it is complete.`);
}

async function doCliLaunch(p) {
  try { await invoke("launch_cli_profile", { name: p.name }); }
  catch (e) { showToast(String(e), "error"); }
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
  if (made) { cliSelected = made.slug; renderCliList(); renderCliDetail(); await doCliLogin(); }
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

// ---------- Open in IDE + Workspaces ----------
let ideFolder = null;
async function openIdeModal() {
  const p = cliCurrent(); if (!p) return;
  if (!p.logged_in) { alert("Log in to this account first."); return; }
  const ides = await invoke("list_ides");
  if (!ides.length) { alert("No VS Code-family IDE found (VS Code, Cursor, Windsurf, Antigravity)."); return; }
  const sel = $("#ide-select");
  sel.innerHTML = ides.map(i => `<option value="${i.id}" data-name="${escapeHtml(i.name)}">${escapeHtml(i.name)}</option>`).join("");
  $("#ide-account").textContent = p.name;
  ideFolder = null;
  $("#ide-folder").textContent = "No folder chosen";
  $("#ide-status").textContent = "";
  $("#ide-modal").classList.remove("hidden");
}
async function doPickFolder() {
  const path = await invoke("pick_folder");
  if (path) { ideFolder = path; $("#ide-folder").textContent = path; }
}
async function doOpenIde() {
  const p = cliCurrent(); if (!p) return;
  if (!ideFolder) { $("#ide-status").textContent = "Choose a project folder first."; return; }
  const sel = $("#ide-select"), opt = sel.options[sel.selectedIndex];
  const ide_id = sel.value, ide_name = opt.dataset.name;
  $("#ide-status").textContent = "Opening…";
  try {
    await invoke("open_in_ide", { account: p.name, ideId: ide_id, projectPath: ideFolder });
    await invoke("save_workspace", { accountSlug: p.slug, accountName: p.name, ideId: ide_id,
      ideName: ide_name, projectPath: ideFolder, now: now() });
  } catch (e) { $("#ide-status").textContent = String(e); return; }
  $("#ide-modal").classList.add("hidden");
  await reloadCli(); // refresh accounts + their nested workspaces
}

// ---------- wire up ----------
window.addEventListener("DOMContentLoaded", () => {
  $("#search").addEventListener("input", (e) => { query = e.target.value; renderList(); });
  $("#cli-search").addEventListener("input", (e) => {
    cliQuery = e.target.value;
    renderCliList();
    renderCliDetail();
  });
  $("#new-side").onclick = openCreate;
  $("#create-cancel").onclick = closeModals;
  $("#create-go").onclick = doCreate;
  $("#edit-cancel").onclick = closeModals;
  $("#del-cancel").onclick = closeModals;
  $("#del-go").onclick = doDelete;
  document.querySelectorAll(".segmented .seg").forEach((b) => b.onclick = () => setView(b.dataset.view));
  $("#cli-new-side").onclick = openCliCreate;
  $("#cli-create-cancel").onclick = closeModals;
  $("#cli-create-go").onclick = doCliCreate;
  // When returning from the Terminal (after `claude auth login`), re-detect login.
  // Throttled (leading-edge): fire on focus, but coalesce rapid focus/blur cycles
  // so we don't re-run the CLI probes more than once per ~800ms.
  let lastCliFocusReload = 0;
  window.addEventListener("focus", () => {
    if (view !== "cli") return;
    const t = Date.now();
    if (t - lastCliFocusReload < 800) return;
    lastCliFocusReload = t;
    reloadCli();
  });
  $("#cli-del-cancel").onclick = closeModals;
  $("#cli-del-go").onclick = doCliDelete;
  $("#ide-pick").onclick = doPickFolder;
  $("#ide-cancel").onclick = () => $("#ide-modal").classList.add("hidden");
  $("#ide-go").onclick = doOpenIde;
  for (const m of document.querySelectorAll(".modal")) {
    m.addEventListener("click", (e) => { if (e.target === m) closeModals(); });
  }
  reload();
});
