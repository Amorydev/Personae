# Windows Test Plan

Personae's Windows code paths are **written from Claude's shipped code + Windows docs but not yet run on a real Windows machine** (the dev host is macOS, so `#[cfg(windows)]` code is not even compiled there). This checklist is the gate before advertising Windows support. Every box is a `TODO(verify)`.

> **First step on a real Windows box: it must COMPILE.** `#[cfg(windows)]` code has never been through `cargo check`/`cargo build` on this repo. Expect to fix compile errors first (imports, `CommandExt`, arg shapes). Build with `cargo build` in `src-tauri`, or the full app with `npm run tauri build` / `cargo tauri build`.

> **Field-confirmed (Windows user, v0.1.0, manual repro — 2026-08):** A Windows user reproduced the multi-account mechanism by hand and confirmed: (1) `CLAUDE_CONFIG_DIR` isolates accounts perfectly; (2) each config dir gets its own `.credentials.json`, `settings.json`, and `history.jsonl`; (3) the OAuth sign-in flow (browser link + code) works in a Windows terminal. This validates the *mechanism* `imp_win` is built on — but it is **not** an end-to-end test of Personae's own launchers/quoting, so every CLI box below still must pass.

> **Session verification (Claude Code, real Windows machine, 2026-08-18):** Ran a genuine (non-GUI) pass — `cargo build`/`cargo test`, plus two temporary `#[ignore]`d probe tests that called the real compiled `cli`/`terminal`/`ide` functions against this machine's actual `%APPDATA%`/`%LOCALAPPDATA%` state (both removed after use, not left in the suite). Boxes below marked ✅ **2026-08-18** are genuinely verified with real evidence from that pass; boxes marked ❌ are explicitly **not** verified and say why — mostly because they require driving the Personae GUI, watching a real console/dialog appear, or completing an interactive OAuth browser flow, none of which this session had tooling for (no click/screen automation). Unmarked boxes are unchanged from before this pass. This machine has: a native `claude.exe` at `~/.local/bin` (no npm shim), VS Code installed (no Cursor/Windsurf/Antigravity), Windows Terminal installed, **no Claude Desktop installed**, and exactly one real signed-in CLI account.

## Environment / tips
- Install the Claude Code CLI (`npm i -g @anthropic-ai/claude-code`, or the native installer). Confirm `where claude` resolves it.
- Override binary resolution for testing with `set CLAUDE_CLI_BIN=C:\path\to\claude.cmd`.
- Per-account config dirs live under `%APPDATA%\ClaudeProfilesCLI\<slug>`; launchers under `%LOCALAPPDATA%\ClaudeProfilesCLI\apps`.

---

## Desktop profiles (`windows.rs`) — pre-existing, still unverified
- [ ] ❌ `claude_found()` locates `Claude.exe`. **Not verified 2026-08-18: Claude Desktop is not installed on this machine** (checked the Squirrel path, the MSIX path, and `where Claude.exe` — none resolved). This whole section needs a box with Claude Desktop actually installed.
- [ ] ❌ `create` writes `<slug>.cmd` + `.name` + tinted `.ico` + Start-Menu `.lnk`; `launch` opens an isolated instance; `quit` kills the right tree; `delete(purge)` removes data. Same reason — no Claude Desktop on this box.
- [ ] ❌ Two profiles run concurrently, each with its own `--user-data-dir`. Same reason.

---

## CLI multi-account (`cli.rs`)
- [x] ✅ **2026-08-18** **Compiles** under `#[cfg(windows)]` — `cargo build --release` and `cargo test` both succeed clean (28 unit tests pass) after the async-command (Phase 2), terminal-picker (Phase 3), and provider-config changes landed in this same session.
- [x] ✅ **2026-08-18 (partial)** `available()` → **true** confirmed live (native `claude.exe` resolved). The **false-when-absent** branch was not exercised — doing so would mean hiding/uninstalling the real `claude` on this machine, too destructive to attempt. ❌ **Not verified:** the **npm `.cmd` shim** path specifically — this machine only has the native `~/.local/bin/claude.exe` install, no npm-installed shim to test against.
- [ ] ❌ `claude_bin()` falling back to the nested `claude-code-win32-x64\claude.exe`: **not exercised for real** — a direct candidate (`~/.local/bin/claude.exe`) resolved first, so the deep-fallback path never triggered. Only the pure path-building helper (`nested_win_claude_exe`) is unit-tested, which was already true before this pass.
- [x] ✅ **2026-08-18** `create` writes `%LOCALAPPDATA%\ClaudeProfilesCLI\apps\<slug>.cmd` + `<slug>.name`, and creates `%APPDATA%\ClaudeProfilesCLI\<slug>` — verified twice: a pre-existing real "Test" account from an earlier real session, and a fresh throwaway account created live this session (see quoting item below) and then deleted.
- [x] ✅ **(mechanism), pre-existing evidence** `login` opens a console, `claude auth login` completes, and `<config_dir>\.credentials.json` is created — confirmed via the pre-existing real "Test" account's genuine `.credentials.json` on disk (real timestamp, real signed-in email). **Caveat: I did not personally trigger `login()` live this session** — spawning it would open a real interactive OAuth console I have no way to complete headlessly (no browser-automation tool), so I left it alone rather than leave a half-finished login window behind.
  - `auth login` subcommand confirmed present on the CLI (`claude auth login` = "Sign in to your Anthropic account"), 2026-08. Re-confirm if the Windows box runs an older CLI.
- [x] ✅ **2026-08-18** After login, `list` reports `logged_in = true` (via `.credentials.json` presence) and the correct email (parsed from `<config_dir>\.claude.json`) — verified live: the probe printed `logged_in=true` and the real parsed email for the pre-existing account.
- [ ] ❌ `launch` opens `claude` in a console **without** re-showing the first-run login/onboarding menu. **Not verified** — confirming this means watching a real console window's actual content, which needs human/screen observation this session doesn't have tooling for. I deliberately did not spawn `launch()` live to avoid leaving a stray interactive console open with no way to close it cleanly from here.
- [ ] ❌ **Concurrency:** two accounts logged in to different Claude logins usable at the same time. **Not verified** — only one real signed-in account exists on this machine; testing this needs a second distinct real Claude login, not available in this session.
- [x] ✅ **2026-08-18** `delete(purge)` removes the `.cmd`, `.name`, and the config dir — verified live: created a throwaway account, confirmed the launcher + config dir existed, called `delete(purge: true)`, confirmed both were gone. (`_login\<slug>.cmd` removal specifically wasn't separately checked since `login()` was never invoked for the throwaway account — no login script was ever written for it to remove.)
- [x] ✅ **2026-08-18 (partial)** **Quoting / batch semantics** — created a real account named `QA Test & Co 100%` and inspected the generated `.cmd`: slug correctly derived (`qa-test-co-100`), `%` correctly doubled to `%%` in the `REM` comment, `&` preserved literally with no line injection, CRLF intact. This is a genuine end-to-end run of the quoting logic, not just the existing pure unit tests.
  - ❌ **Not verified:** `claude` resolved at a path *with spaces* — the real resolved path on this machine (`~/.local/bin/claude.exe`) has no spaces, so this specific sub-case never got exercised.
  - ❌ **Not verified:** `login()`'s empty-title-arg behavior and `launch()`'s bare-arg REPL-opens behavior — both require spawning and watching a real console, not attempted this session (see reasoning above).

---

## IDE / Workspaces (`ide.rs`)
- [x] ✅ **2026-08-18** **Compiles** under `#[cfg(windows)]` — part of the same clean `cargo build`/`cargo test` pass above.
- [x] ✅ **2026-08-18 (partial)** `list_ides()` — VS Code correctly detected live (probe returned `["vscode"]`, matching `where code` on this machine). ❌ **Not verified:** Cursor, Windsurf, and Antigravity detection — none of those are installed here, so their candidate-path logic never ran for real. **Antigravity's install layout + CLI name are still unconfirmed** — verify and adjust the candidate paths on a box that has it.
- [ ] ❌ `pick_folder()` shows the PowerShell `FolderBrowserDialog`. **Not verified** — this is an interactive native dialog a human has to click through; no way to drive it headlessly this session.
- [ ] ❌ `open_in_ide` writes `.vscode/settings.json` / `.vscode/tasks.json` correctly. **Not verified** — would need an actual run through a real project folder, not attempted.
- [ ] ❌ Opening the folder auto-runs `claude` in a terminal signed in to the chosen account. **Not verified**, same reason.
- [ ] ❌ **⚠ Launch quoting (highest-risk item):** VS Code shim path with spaces + project path with spaces. **Not verified** — and notably **the space-in-shim-path precondition doesn't even hold on this machine** (`C:\Users\DevStar\AppData\Local\Programs\Microsoft VS Code\bin\code.cmd` has no spaces), so this box needs a different Windows box (or the default per-machine install at `C:\Program Files\...`) to test meaningfully at all.
- [ ] `ide::imp_win::cli_config_dir` byte-matches `cli::imp_win::config_dir` — both resolve to `appdata().join("ClaudeProfilesCLI").join(slug)` by direct source inspection (unchanged from before this pass); not re-verified live beyond reading the code again.
- [ ] ❌ Workspaces persist across app restarts. **Not verified** — needs an actual create → restart → reopen pass through the GUI, not attempted.

---

## Cross-cutting
- [ ] ❌ **Console flash:** `CREATE_NO_WINDOW` / `run_hidden` producing zero visible flashes. **Not verified** — this needs a human watching the screen in real time during `available`/`create`/`list`/IDE-detection calls; no screen-observation tooling available this session. Code inspection still shows the `CREATE_NO_WINDOW` / `run_hidden` calls in place, but that's not the same as confirming no residual flash.
- [ ] **Branch note:** the CLI multi-account work (PR #4, `feat/cli-multi-account`) is merged into `main`; `main` is the source of truth. Unchanged by this pass — not re-verified, no reason to expect it changed.
- [ ] After all boxes pass: flip the landing page CLI badge from `macOS` to `macOS · Windows` and update the FAQ ("The Claude Code (CLI) accounts are macOS for now."). **Still blocked** — several boxes above are explicitly unverified or rejected, especially the whole Desktop-profiles section (no Claude Desktop on any box tested so far) and the IDE launch-quoting item (highest-risk, and this session's box doesn't even reproduce the risky precondition).
