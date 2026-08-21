import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const read = (f) => readFile(join(here, f), "utf8");

// Single source of truth for the advertised version: LANDING_VERSION (set by
// the deploy workflow from the latest release tag) wins, otherwise the repo's
// package.json version. Both feed the {{VERSION}} placeholders in the sources
// so the landing can never drift from the release the way a hand-edited string
// did. Leading "v" is stripped so a tag ref (v0.1.13) and a bare version both work.
const pkg = JSON.parse(await read("../package.json"));
const version = (process.env.LANDING_VERSION || pkg.version).replace(/^v/, "");

const [head, body, tail] = await Promise.all([
  read("_seo-head.html"),
  read("index.html"),
  read("_seo-tail.html"),
]);

const out = (head + body + tail).replaceAll("{{VERSION}}", version);

await writeFile(join(here, "site/index.html"), out);
console.log(`landing/site/index.html rebuilt (version ${version})`);
