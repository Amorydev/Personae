# Windows Test Plan

Personae's Windows code paths are **written from Claude's shipped code + Windows docs but not yet run on a real Windows machine** (the dev host is macOS, so `#[cfg(windows)]` code is not even compiled there). This checklist is the gate before advertising Windows support. Every box is a `TODO(verify)`.

> **First step on a real Windows box: it must COMPILE.** `#[cfg(windows)]` code has never been through `cargo check`/`cargo build` on this repo. Expect to fix compile errors first (imports, `CommandExt`, arg shapes). Build with `cargo build` in `src-tauri`, or the full app with `npm run tauri build` / `cargo tauri build`.

## Environment / tips
- Install the Claude Code CLI (`npm i -g @anthropic-ai/claude-code`, or the native installer). Confirm `where claude` resolves it.
- Override binary resolution for testing with `set CLAUDE_CLI_BIN=C:\path\to\claude.cmd`.
- Per-account config dirs live under `%APPDATA%\ClaudeProfilesCLI\<slug>`; launchers under `%LOCALAPPDATA%\ClaudeProfilesCLI\apps`.

---

## Desktop profiles (`windows.rs`) — pre-existing, still unverified
- [ ] `claude_found()` locates `Claude.exe` (Squirrel `%LOCALAPPDATA%\AnthropicClaude\app-<ver>`, MSIX `%LOCALAPPDATA%\Microsoft\WindowsApps\Claude.exe`, registry, running process).
- [ ] `create` writes `<slug>.cmd` + `.name` + tinted `.ico` + Start-Menu `.lnk`; `launch` opens an isolated instance; `quit` kills the right tree; `delete(purge)` removes data.
- [ ] Two profiles run concurrently, each with its own `--user-data-dir`.

---

## CLI multi-account (`cli.rs`)
- [ ] **Compiles** under `#[cfg(windows)]`.
- [ ] `available()` → true when `claude` is installed (test BOTH the npm `.cmd` shim and, if present, a native `.exe`); false when absent (empty-state shows "Install the `claude` CLI…").
- [ ] `create` writes `%LOCALAPPDATA%\ClaudeProfilesCLI\apps\<slug>.cmd` + `<slug>.name`, and creates `%APPDATA%\ClaudeProfilesCLI\<slug>`.
- [ ] `login` opens a console (`_login\<slug>.cmd` via `cmd /C start "" "<script>"`), `claude auth login` completes, and **`<config_dir>\.credentials.json` is created** — this is the whole premise (no Keychain on Windows; the credential store falls back to this plaintext file).
- [ ] After login, `list` reports `logged_in = true` (detected purely by `.credentials.json` presence) and the correct email (parsed from `<config_dir>\.claude.json`).
- [ ] `launch` opens `claude` in a console **without** re-showing the first-run login/onboarding menu (confirms `mark_onboarded` wrote `hasCompletedOnboarding` into `.claude.json`).
- [ ] **Concurrency:** two accounts logged in to different Claude logins are both usable at the same time (open both launchers); their emails differ; neither sees the other's history.
- [ ] `delete(purge)` removes the `.cmd`, `.name`, `_login\<slug>.cmd`, and the config dir (the login goes with the file — nothing left behind).
- [ ] **Quoting / batch semantics** (the launcher is CRLF, uses `call "<bin>" %*`, escapes `%`→`%%`):
  - [ ] account name with spaces, `&`, and `%`.
  - [ ] `claude` resolved at a path with spaces.
  - [ ] `login()` `cmd /C start "" "<script>"` — empty title arg behaves (does not consume the path as the title).
  - [ ] `launch()` `cmd /C start "" cmd /K "<launcher>"` — the fixup passes the launcher as a **bare** arg (let `Command` quote once); confirm the account REPL actually opens (a path with spaces was the failure mode of the pre-fix double-quoting).

---

## IDE / Workspaces (`ide.rs`)
- [ ] **Compiles** under `#[cfg(windows)]` (note the `use std::os::windows::process::CommandExt;` for `raw_arg`/`creation_flags` in `open_in_ide`).
- [ ] `list_ides()` finds installed VS Code / Cursor / Windsurf / Antigravity (via `where <cli>` then candidate `.cmd` paths). **Antigravity's install layout + CLI name are unconfirmed** — verify and adjust the candidate paths.
- [ ] `pick_folder()` shows the PowerShell `FolderBrowserDialog` (STA); choosing a folder returns its path; **Cancel → no error** (empty output → `Ok(None)`); a genuine PowerShell failure surfaces as an error (not silently treated as cancel).
- [ ] `open_in_ide` writes `.vscode/settings.json` with `terminal.integrated.env.windows.CLAUDE_CONFIG_DIR` and a `.vscode/tasks.json` `folderOpen` task (`type: "shell"`, `command` = resolved claude, `options.env.CLAUDE_CONFIG_DIR`).
- [ ] Opening the folder **auto-runs `claude` in a terminal signed in to the chosen account**; a manually-opened integrated terminal also has the right `CLAUDE_CONFIG_DIR`.
- [ ] **⚠ Launch quoting (highest-risk item):** with the **default VS Code shim** `C:\Program Files\Microsoft VS Code\bin\code.cmd` (has spaces) **and** a project path that also has spaces (e.g. `C:\Users\me\My Projects\app`), the folder still opens. The fix builds `cmd /C ""<cli>" "<path>""` via `raw_arg` (outer-quote idiom) — confirm it works for: no-space/no-space, space/no-space, no-space/space, and space/space.
- [ ] The IDE's terminal account dir (`ide::imp_win::cli_config_dir`) matches the CLI engine's (`cli::imp_win::config_dir`) — both `%APPDATA%\ClaudeProfilesCLI\<slug>` (verified byte-equal in review; confirm live).
- [ ] Workspaces persist across app restarts in `%APPDATA%\ClaudeProfilesCLI\workspaces.json`; reopen re-applies activation and launches; delete removes the row.

---

## Cross-cutting
- [ ] **Console flash:** on this branch (`feat/cli-multi-account`) `platform::run()` does **not** set `CREATE_NO_WINDOW`, so background probes (`where claude`, `powershell` in `detect_ides`/`pick_folder`, `cli::login`/`launch`) will briefly flash a console window. `main`'s `platform.rs` already adds `CREATE_NO_WINDOW` to `run()` — bring that over (or reconcile onto `main`) to suppress the flashes. (`open_in_ide`'s launch already sets `CREATE_NO_WINDOW` directly after the review fixup.)
- [ ] **Branch note:** this work is committed on `feat/cli-multi-account`, which is behind `main` (`main` has the newer `platform.rs` with `CREATE_NO_WINDOW` and a rewritten Desktop `windows.rs`). Decide how to reconcile before shipping.
- [ ] After all boxes pass: flip the landing page CLI badge from `macOS` to `macOS · Windows` and update the FAQ ("The Claude Code (CLI) accounts are macOS for now.").
