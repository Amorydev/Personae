#!/usr/bin/env node
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const exeName = process.platform === "win32" ? "claude-profiles-tauri.exe" : "claude-profiles-tauri";
const exePath = join(__dirname, "..", "src-tauri", "target", "release", exeName);

if (!existsSync(exePath)) {
  console.error(`Personae isn't built yet. Run \`npm run deploy\` first.\n(expected ${exePath})`);
  process.exit(1);
}

const child = spawn(exePath, [], { detached: true, stdio: "ignore" });
child.unref();
