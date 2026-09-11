import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (file) => readFile(path.join(root, file), "utf8");
const [packageJson, tauriJson, readme, readmeZh, docsIndex, architecture, design, plan] = await Promise.all([
  read("package.json").then(JSON.parse),
  read("src-tauri/tauri.conf.json").then(JSON.parse),
  read("README.md"),
  read("README.zh-CN.md"),
  read("docs/README.md"),
  read("docs/ARCHITECTURE.md"),
  read("DESIGN.md"),
  read("docs/PLAN.md"),
]);

assert.equal(tauriJson.version, packageJson.version, "package and Tauri versions drifted");
assert.equal(tauriJson.productName, "MetaClean");
assert.match(packageJson.description, /local-first file privacy cleaner/u);
assert.match(readme, /portable ZIP/u, "English download table must name portable packages");
assert.match(readmeZh, /便携 ZIP/u, "Chinese download table must name portable packages");
assert.match(readme, /PDF, Office and text use format-aware rewrites/u);
assert.match(readmeZh, /PDF、Office 与文本按各自结构安全重写/u);
assert.match(docsIndex, /ARCHITECTURE\.md/u);
assert.match(architecture, /Non-negotiable invariants/u);
assert.match(architecture, /pathIdentity/u);
assert.match(design, /wcb\.txt/u);
assert.match(plan, /Word 与 WPS/u);

for (const [name, contents] of Object.entries({ README: readme, "README.zh-CN": readmeZh, DESIGN: design, "docs/README": docsIndex, "docs/ARCHITECTURE": architecture, "docs/PLAN": plan })) {
  assert.doesNotMatch(contents, /[A-Z]:\\Users\\/u, `${name} publishes a workstation path`);
}

console.log("Verified product descriptions, version metadata, documentation entry points and external-validation boundaries.");
