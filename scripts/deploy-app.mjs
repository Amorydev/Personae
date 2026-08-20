#!/usr/bin/env node
/**
 * Build Personae and replace the copy installed on this machine, so the
 * installed app matches the working tree.
 *
 *     node scripts/deploy-app.mjs [--dry-run] [--yes] [--skip-build] [--dmg]
 *
 * Windows builds the NSIS `*-setup.exe` and runs it silently. Its install dir
 * is %LOCALAPPDATA%\Personae, which is ALSO where the app keeps its CLI
 * launchers (crates/engine/src/cli.rs:628) — and those launchers *are* the
 * account registry, since cli.rs:1184 discovers accounts by globbing
 * CLI\apps\*.cmd. Were they ever removed, every CLI account would vanish from
 * the UI while the credentials sat untouched in %APPDATA%\Personae\CLI.
 *
 * Two upstream details make that safe today: under /S the reinstall page never
 * runs, so the old uninstaller is never invoked and the install is a plain
 * overwrite; and the uninstaller's own `RMDir "$INSTDIR"` carries no /r, so it
 * could not remove a non-empty tree regardless. Neither is a guarantee we own,
 * so the CLI\ tree is still copied aside and restored around the install.
 * The flip side of overwriting: files dropped between versions are never
 * cleaned up, so run uninstall.exe by hand occasionally to clear stale ones.
 *
 * macOS needs no such dance: it replaces /Applications/Personae.app, while
 * launchers live in ~/Applications/Personae/CLI (cli.rs:505) and config in
 * ~/Library/Application Support/Personae/CLI (cli.rs:507) — both outside the
 * bundle.
 */
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
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

/** SHA-256 of a file, for proving the installed binary is the built one. */
function sha256(p) {
  return createHash("sha256").update(readFileSync(p)).digest("hex");
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
  // Captured before anything runs: it decides the installer flags below.
  const upgrading = existsSync(installedExe);

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
  // Ask for a clean exit before handing over to the installer, which under /S
  // goes straight to TerminateProcess with no WM_CLOSE and no wait — enough to
  // tear an in-flight write to a profile's JSON. Force is the fallback only.
  const q = (s) => s.replace(/'/g, "''");
  const devExe = join(TARGET, "release", "claude-profiles-tauri.exe");
  const stopCmd = "powershell -NoProfile -Command \""
    + "$ps = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'claude-profiles-tauri.exe' -and ("
    + `$_.ExecutablePath -like '${q(installDir)}\\*' -or $_.ExecutablePath -eq '${q(devExe)}'`
    + ") }); "
    + "foreach ($c in $ps) { $p = Get-Process -Id $c.ProcessId -ErrorAction SilentlyContinue; if ($p) { $null = $p.CloseMainWindow() }; $c.ProcessId }; "
    + "if ($ps) { Start-Sleep -Milliseconds 1500; foreach ($c in $ps) { Stop-Process -Id $c.ProcessId -Force -ErrorAction SilentlyContinue } }\"";
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

  // /UPDATE on an upgrade skips the WebView2 bootstrapper section, which shells
  // out to Microsoft's installer and needs elevation — under /S its UAC prompt
  // is the only UI and blocks forever. A first install uses plain /S so that
  // shortcuts still get created (/UPDATE suppresses them).
  // Never /D= : the template restores the recorded install dir, and passing /D
  // would create a second install alongside a non-default one.
  const flags = upgrading ? "/S /UPDATE" : "/S";
  // Snapshot the outgoing binary so the summary can say whether it actually
  // changed; /S overwrites in place without version-checking and still exits 0.
  const beforeHash = upgrading ? sha256(installedExe) : null;
  step(`Installing (silent, ${flags})`);
  const code = run(`"${setup}" ${flags}`, { allowFail: true });
  // 0 success, 1 user cancel, 2 script abort — the template sets no others.
  if (code !== 0) info(`installer exit code ${code}`);

  // Restore whatever the uninstall step removed. Existing files win, so a
  // launcher the fresh install wrote is never clobbered by a stale copy.
  if (backup) {
    cpSync(backup, launcherTree, { recursive: true, force: false, errorOnExist: false });
    step("Restored CLI launchers");
    info(`${countLaunchers(appsDir)} account(s) present at ${appsDir}`);
  }

  // Neither mtime nor the exit code proves anything here: NSIS's `File`
  // preserves the source timestamp, so the installed mtime is the BUILD time;
  // and /S overwrites in place without version-checking, so re-running an old
  // installer still exits 0.
  step("Verifying");
  if (!existsSync(installedExe)) die(`install finished but ${installedExe} is missing`);
  const installedSize = statSync(installedExe).size;
  info(installedExe);
  info(`${(installedSize / 1048576).toFixed(1)} MiB`);

  const want = JSON.parse(readFileSync(join(REPO_ROOT, "src-tauri", "tauri.conf.json"), "utf8")).version;
  const ver = regRead("DisplayVersion");
  if (ver) info(`registered version ${ver}`);
  if (ver && want && ver !== want) die(`registry reports ${ver} but this tree builds ${want}`);

  // Size rather than hash, because the two differ by design: `tauri build`
  // stamps a 3-byte bundle-type marker ("NSS") into the copy it hands the
  // installer and leaves "UNK" in the binary at target/release. Same build,
  // three bytes apart — so a hash compare here false-fails every time, while
  // size still catches a truncated or half-written install.
  const builtExe = join(TARGET, "release", "claude-profiles-tauri.exe");
  if (existsSync(builtExe)) {
    const builtSize = statSync(builtExe).size;
    if (builtSize !== installedSize) {
      die(`installed binary is ${installedSize} bytes but the build is ${builtSize} — the install did not take effect`);
    }
    info("size matches the build in src-tauri/target");
  }
  if (beforeHash) {
    info(sha256(installedExe) === beforeHash ? "binary unchanged (already current)" : "binary replaced");
  }
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

  // A running app cannot be safely replaced. Anchor the pattern to the start of
  // the command line AND to the install path: the executable inside the bundle
  // is `claude-profiles-tauri` (mainBinaryName is unset, so it falls back to the
  // cargo bin name) and p_comm truncates at 16 chars, which rules out pgrep -x;
  // meanwhile an unanchored -f pattern would match this script's own shell
  // wrapper and the freshly built copy under src-tauri/target.
  const PAT = `^${dest}/Contents/MacOS/`;
  const RUNNING = `pgrep -f '${PAT}'`;
  if (DRY_RUN) {
    step("Would quit any running Personae");
  } else if (capture(`${RUNNING} || true`)) {
    step("Quitting running Personae");
    // Address it by bundle id and guard on `is running`; a bare `quit app`
    // would launch the app rather than close it.
    run(`osascript -e 'tell application id "com.amoryzenith.personae" to if it is running then quit' >/dev/null 2>&1 || true`, { allowFail: true });
    run(`for i in $(seq 1 25); do ${RUNNING} >/dev/null 2>&1 || break; sleep 0.2; done`, { allowFail: true });
    run(`pkill -f '${PAT}' || true`, { allowFail: true });
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
    // Mount at a private mountpoint rather than parsing hdiutil's tabular
    // output or guessing under /Volumes — a volume name containing spaces or
    // tabs breaks both, and /Volumes collides with an already-mounted copy.
    const mnt = capture("mktemp -d");
    if (!mnt) die("could not create a temporary mountpoint");
    run(`hdiutil attach "${dmg}" -nobrowse -readonly -mountpoint "${mnt}"`);
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
  // The bundle executable is the cargo bin name, not the product name.
  const destBin = join(dest, "Contents", "MacOS", "claude-profiles-tauri");
  if (!existsSync(destBin)) die(`install finished but ${destBin} is missing`);
  const ver = capture(`defaults read "${dest}/Contents/Info.plist" CFBundleShortVersionString`);
  if (ver) info(`installed version ${ver}`);
  // Size, not mtime: ditto preserves the source mtime, so an unchanged mtime
  // proves nothing. Hash only informs — on Windows the same comparison differs
  // by three bytes of bundle-type marker between the build tree and the
  // installed copy, and that behaviour is unverified here, so a mismatch is
  // reported rather than treated as failure.
  const builtApp = findNewestDir(TARGET, "Personae.app");
  const builtBin = builtApp && join(builtApp, "Contents", "MacOS", "claude-profiles-tauri");
  if (builtBin && existsSync(builtBin)) {
    const a = statSync(builtBin).size, b = statSync(destBin).size;
    if (a !== b) die(`installed binary is ${b} bytes but the build is ${a} — the install did not take effect`);
    info("size matches the build in src-tauri/target");
    if (sha256(builtBin) !== sha256(destBin)) info("note: bytes differ from the build (expected if a bundle-type marker is stamped)");
  }
  info(dest);
  console.log("\nDone. The installed Personae now matches this working tree.");
}

/* --------------------------------------------------------------------- main */

if (process.platform === "win32") await deployWindows();
else if (process.platform === "darwin") await deployMacOS();
else die(`unsupported platform '${process.platform}' — this deploys the desktop app on Windows and macOS only`);
