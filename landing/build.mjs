import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const read = (f) => readFile(join(here, f), "utf8");

const [head, body, tail] = await Promise.all([
  read("_seo-head.html"),
  read("index.html"),
  read("_seo-tail.html"),
]);

await writeFile(join(here, "site/index.html"), head + body + tail);
console.log("landing/site/index.html rebuilt");
