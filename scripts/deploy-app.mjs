#!/usr/bin/env node
/**
 * Build Personae and replace the copy installed on this machine, so the
 * installed app matches the working tree.
 *
 *     node scripts/deploy-app.mjs [--dry-run] [--yes] [--skip-build] [--dmg]
 *
 * Windows builds the NSIS `*-setup.exe` and runs it silently. That installer
 * uninstalls the previous version first, and its install dir is
 * %LOCALAPPDATA%\Personae — which is ALSO where the app keeps its CLI
 * launchers (crates/engine/src/cli.rs:628). Those launchers *are* the account
 * registry: cli.rs:1184 discovers accounts by globbing CLI\apps\*.cmd, so
 * losing them makes every CLI account vanish from the UI even though the
 * credentials themselves survive in %APPDATA%\Personae\CLI. The whole CLI\
 * tree is therefore copied aside before the installer runs and restored after.
 *
 * macOS needs no such dance: it replaces /Applications/Personae.app, while
 * launchers live in ~/Applications/Personae/CLI (cli.rs:505) and config in
 * ~/Library/Application Support/Personae/CLI (cli.rs:507) — both outside the
 * bundle.
 */
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline/promises";

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TARGET = join(REPO_ROOT, "src-tauri", "target");

let DRY_RUN = 0, ASSUME_YES = 0, SKIP_BUILD = 0, USE_DMG = 0;

function die(msg) { console.error(`error: ${msg}`); process.exit(1); }
function step(msg) { console.log(`==> ${msg}`); }
function info(msg) { console.log(`    ${msg}`); }

function usage() {
  console.log(`Build Personae and replace the copy installed on this machine.

    node scripts/deploy-app.mjs [--dry-run] [--yes] [--skip-build] [--dmg]

  --dry-run     Show what would happen; build nothing, install nothing.
  --yes, -y     Skip the confirmation prompt.
  --skip-build  Reuse the installer/bundle already in src-tauri/target.
  --dmg         macOS only: also build the .dmg and install from it, instead
                of copying the built .app straight into /Applications.

Windows installs to %LOCALAPPDATA%\\Personae via the NSIS installer, and the
CLI launcher tree under that directory is backed up and restored around it,
because those launchers are the only record of which CLI accounts exist.
macOS replaces /Applications/Personae.app.

Set PERSONAE_INSTALL_DIR to override the Windows install directory.`);
}

for (const arg of process.argv.slice(2)) {
  if (arg === "--dry-run") DRY_RUN = 1;
  else if (arg === "--yes" || arg === "-y") ASSUME_YES = 1;
  else if (arg === "--skip-build") SKIP_BUILD = 1;
  else if (arg === "--dmg") USE_DMG = 1;
  else if (arg === "-h" || arg === "--help") { usage(); process.exit(0); }
  else die(`unknown argument: ${arg} (try --help)`);
}

/** Run a command through the shell, inheriting stdio. Dies on non-zero exit. */
function run(cmd, { allowFail = false } = {}) {
  const r = spawnSync(cmd, { cwd: REPO_ROOT, stdio: "inherit", shell: true });
  if (r.error) die(`failed to run ${cmd}: ${r.error.message}`);
  if (!allowFail && r.status !== 0) die(`${cmd} exited with code ${r.status}`);
  return r.status;
}

/** Run a command and capture stdout. Never dies; returns "" on failure. */
function capture(cmd) {
  const r = spawnSync(cmd, { cwd: REPO_ROOT, encoding: "utf8", shell: true });
  return r.status === 0 && r.stdout ? r.stdout.trim() : "";
}

/** Newest file under `root` whose basename matches `re`. */
function findNewest(root, re) {
  if (!existsSync(root)) return null;
  let best = null;
  const walk = (dir) => {
    let entries;
    try { entries = readdirSync(dir, { withFileTypes: true }); } catch { return; }
    for (const e of entries) {
      const p = join(dir, e.name);
      if (e.isDirectory()) walk(p);
      else if (re.test(e.name)) {
        const t = statSync(p).mtimeMs;
        if (!best || t > best.mtime) best = { path: p, mtime: t };
      }
    }
  };
  walk(root);
  return best ? best.path : null;
}

/** Newest directory under `root` named exactly `name`. */
function findNewestDir(root, name) {
  if (!existsSync(root)) return null;
  let best = null;
  const walk = (dir) => {
    let entries;
    try { entries = readdirSync(dir, { withFileTypes: true }); } catch { return; }
    for (const e of entries) {
      if (!e.isDirectory()) continue;
      const p = join(dir, e.name);
      if (e.name === name) {
        const t = statSync(p).mtimeMs;
        if (!best || t > best.mtime) best = { path: p, mtime: t };
      } else walk(p);
    }
  };
  walk(root);
  return best ? best.path : null;
}

function countLaunchers(appsDir) {
  if (!existsSync(appsDir)) return 0;
  return readdirSync(appsDir).filter((f) => f.endsWith(".cmd")).length;
}

async function confirm(question) {
  if (DRY_RUN || ASSUME_YES) return true;
  if (!process.stdin.isTTY) die("not a TTY — rerun with --yes to confirm non-interactively");
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  const reply = (await rl.question(`${question} [y/N] `)).trim().toLowerCase();
  rl.close();
  return reply === "y" || reply === "yes";
}

/* ------------------------------------------------------------------ Windows */

/** Where the NSIS installer records itself, for a currentUser install. */
const UNINSTALL_KEY =
  "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Personae";

/** Read one REG_SZ value from the uninstall key; "" when absent. */
function regRead(name) {
  const out = capture(`reg query "${UNINSTALL_KEY}" /v ${name}`);
  const m = out.match(new RegExp(`${name}\\s+REG_SZ\\s+(.+)`));
  return m ? m[1].trim().replace(/^"|"$/g, "") : "";
}

async function deployWindows() {
  // Prefer what the previous install actually recorded over guessing the
  // default, so a non-default install location is upgraded in place.
  const installDir = process.env.PERSONAE_INSTALL_DIR
    || regRead("InstallLocation")
    || join(process.env.LOCALAPPDATA || "", "Personae");
  const installedExe = join(installDir, regRead("MainBinaryName") || "claude-profiles-tauri.exe");
  const launcherTree = join(installDir, "CLI");
  const appsDir = join(launcherTree, "apps");

  step("Plan");
  info("platform     Windows");
  info(`install dir  ${installDir}`);
  info("installer    NSIS *-setup.exe, silent (/S)");
  info(`safeguard    ${launcherTree} backed up and restored around the install`);
  if (existsSync(appsDir)) info(`             ${countLaunchers(appsDir)} CLI account(s) to preserve`);
  if (!existsSync(installDir)) info("note         install dir does not exist — this is a first install");
  if (DRY_RUN) info("mode         DRY RUN — nothing will be built or installed");
  console.log();

  if (!(await confirm(`Rebuild Personae and replace the install at ${installDir}?`))) {
    console.log("Aborted.");
    process.exit(1);
  }

  if (DRY_RUN) {
    step("Would build: npx tauri build --bundles nsis");
  } else if (!SKIP_BUILD) {
    step("Building NSIS installer");
    run("npx tauri build --bundles nsis");
  }

  const setup = findNewest(TARGET, /-setup\.exe$/i);
  if (!setup) {
    if (DRY_RUN) { info("(no installer on disk yet — it would come from the build above)"); return; }
    die("no *-setup.exe found under src-tauri/target — did the bundle step run?");
  }
  step("Installer");
  info(setup);
  info(`${(statSync(setup).size / 1048576).toFixed(1)} MiB`);

  // Back up the launcher tree: it lives inside $INSTDIR, and those launchers
  // are the only record of which CLI accounts exist (cli.rs:1184).
  let backup = null;
  if (existsSync(launcherTree)) {
    if (DRY_RUN) {
      step(`Would back up ${launcherTree}`);
    } else {
      backup = join(mkdtempSync(join(tmpdir(), "personae-deploy-")), "CLI");
      cpSync(launcherTree, backup, { recursive: true });
      step(`Backed up CLI launchers (${countLaunchers(join(backup, "apps"))} account(s))`);
      info(backup);
    }
  }

  // Close a running instance, scoped by executable path to the install dir or
  // the build tree — never any other process that happens to share a name.
  const q = (s) => s.replace(/'/g, "''");
  const devExe = join(TARGET, "release", "claude-profiles-tauri.exe");
  const stopCmd = "powershell -NoProfile -Command \""
    + "Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'claude-profiles-tauri.exe' -and ("
    + `$_.ExecutablePath -like '${q(installDir)}\\*' -or $_.ExecutablePath -eq '${q(devExe)}'`
    + ") } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; $_.ProcessId }\"";
  if (DRY_RUN) {
    step("Would close any running Personae inside the install dir");
  } else {
    const stopped = capture(stopCmd);
    if (stopped) {
      step("Closed running Personae");
      info(`pid ${stopped.split(/\s+/).filter(Boolean).join(", ")}`);
    }
  }

  if (DRY_RUN) {
    step("Would run the installer silently, restore launchers, then verify");
    return;
  }

  step("Installing (silent)");
  // No /D= — the NSIS installer reuses the recorded install dir on upgrade.
  const code = run(`"${setup}" /S`, { allowFail: true });
  if (code !== 0) info(`installer exit code ${code}`);

  // Restore whatever the uninstall step removed. Existing files win, so a
  // launcher the fresh install wrote is never clobbered by a stale copy.
  if (backup) {
    cpSync(backup, launcherTree, { recursive: true, force: false, errorOnExist: false });
    step("Restored CLI launchers");
    info(`${countLaunchers(appsDir)} account(s) present at ${appsDir}`);
  }

  step("Verifying");
  if (!existsSync(installedExe)) die(`install finished but ${installedExe} is missing`);
  const st = statSync(installedExe);
  const ageMin = (Date.now() - st.mtimeMs) / 60000;
  info(installedExe);
  info(`${(st.size / 1048576).toFixed(1)} MiB, modified ${ageMin < 10 ? "just now" : ageMin.toFixed(0) + " min ago"}`);
  const ver = regRead("DisplayVersion");
  if (ver) info(`registered version ${ver}`);
  if (ageMin > 10) die("the installed binary was not updated — the installer may have failed silently");
  console.log("\nDone. The installed Personae now matches this working tree.");
}

/* -------------------------------------------------------------------- macOS */

async function deployMacOS() {
  const dest = "/Applications/Personae.app";
  const bundles = USE_DMG ? "app,dmg" : "app";

  step("Plan");
  info("platform     macOS");
  info(`install dir  ${dest}`);
  info(`source       ${USE_DMG ? "mounted .dmg" : "built .app bundle"}`);
  info("safeguard    none needed — launchers (~/Applications/Personae/CLI) and");
  info("             config (~/Library/Application Support/Personae/CLI) live");
  info("             outside the bundle and are not touched");
  if (DRY_RUN) info("mode         DRY RUN — nothing will be built or installed");
  console.log();

  if (!(await confirm(`Rebuild Personae and replace ${dest}?`))) {
    console.log("Aborted.");
    process.exit(1);
  }

  if (DRY_RUN) {
    step(`Would build: npx tauri build --bundles ${bundles}`);
  } else if (!SKIP_BUILD) {
    step(`Building bundle (${bundles})`);
    run(`npx tauri build --bundles ${bundles}`);
  }

  // A running app cannot be safely replaced; ask it to quit, then insist.
  const RUNNING = "pgrep -f '/Personae.app/Contents/MacOS/'";
  if (DRY_RUN) {
    step("Would quit any running Personae");
  } else if (capture(`${RUNNING} || true`)) {
    step("Quitting running Personae");
    run(`osascript -e 'quit app "Personae"' || true`, { allowFail: true });
    run(`for i in 1 2 3 4 5; do ${RUNNING} >/dev/null || break; sleep 1; done`, { allowFail: true });
    run("pkill -f '/Personae.app/Contents/MacOS/' || true", { allowFail: true });
  }

  let source;
  let mounted = null;
  if (USE_DMG) {
    const dmg = findNewest(TARGET, /\.dmg$/i);
    if (!dmg) {
      if (DRY_RUN) { info("(no .dmg on disk yet — it would come from the build above)"); return; }
      die("no .dmg found under src-tauri/target");
    }
    step("Installer");
    info(dmg);
    if (DRY_RUN) { step(`Would mount it and copy Personae.app to ${dest}`); return; }
    const mnt = capture(`hdiutil attach "${dmg}" -nobrowse -readonly | tail -1 | cut -f3-`);
    if (!mnt || !existsSync(mnt)) die(`could not mount ${dmg}`);
    mounted = mnt;
    source = join(mnt, "Personae.app");
    if (!existsSync(source)) {
      run(`hdiutil detach "${mnt}" -quiet || true`, { allowFail: true });
      die(`mounted ${dmg} but Personae.app was not inside it`);
    }
  } else {
    source = findNewestDir(TARGET, "Personae.app");
    if (!source) {
      if (DRY_RUN) { info("(no .app on disk yet — it would come from the build above)"); return; }
      die("no Personae.app found under src-tauri/target");
    }
    step("Bundle");
    info(source);
    if (DRY_RUN) { step(`Would replace ${dest}`); return; }
  }

  try {
    step(`Replacing ${dest}`);
    if (existsSync(dest)) rmSync(dest, { recursive: true, force: true });
    // ditto, not cp -R: it preserves extended attributes, symlinks and the
    // code signature inside the bundle.
    run(`ditto "${source}" "${dest}"`);
  } finally {
    if (mounted) run(`hdiutil detach "${mounted}" -quiet || true`, { allowFail: true });
  }

  step("Verifying");
  if (!existsSync(join(dest, "Contents", "MacOS"))) die(`install finished but ${dest} looks incomplete`);
  const ver = capture(`defaults read "${dest}/Contents/Info.plist" CFBundleShortVersionString`);
  if (ver) info(`installed version ${ver}`);
  info(dest);
  console.log("\nDone. The installed Personae now matches this working tree.");
}

/* --------------------------------------------------------------------- main */

if (process.platform === "win32") await deployWindows();
else if (process.platform === "darwin") await deployMacOS();
else die(`unsupported platform '${process.platform}' — this deploys the desktop app on Windows and macOS only`);
