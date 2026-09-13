import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (file) => readFile(path.join(root, file), "utf8");
const [packageJson, tauriJson, readme, readmeZh, docsIndex, architecture, design, plan, validation, docsConfig, docsHome, customCss] = await Promise.all([
  read("package.json").then(JSON.parse),
  read("src-tauri/tauri.conf.json").then(JSON.parse),
  read("README.md"),
  read("README.zh-CN.md"),
  read("docs/README.md"),
  read("docs/ARCHITECTURE.md"),
  read("DESIGN.md"),
  read("docs/PLAN.md"),
  read("VALIDATION.md"),
  read("docs/.vitepress/config.mts"),
  read("docs/index.md"),
  read("docs/.vitepress/theme/custom.css"),
]);

assert.equal(tauriJson.version, packageJson.version, "package and Tauri versions drifted");
assert.equal(tauriJson.productName, "MetaClean");
assert.match(packageJson.description, /local-first file privacy cleaner/u);
assert.match(readme, /portable ZIP/u, "English download table must name portable packages");
assert.match(readmeZh, /便携 ZIP/u, "Chinese download table must name portable packages");
assert.match(readme, /PDF, Office and UTF-8\/BOM-marked UTF-16 text use format-aware rewrites/u);
assert.match(readmeZh, /PDF、Office 与 UTF-8\/带 BOM 的 UTF-16 文本按各自结构安全重写/u);
assert.match(docsIndex, /ARCHITECTURE\.md/u);
assert.match(architecture, /Non-negotiable invariants/u);
assert.match(architecture, /pathIdentity/u);
assert.match(design, /wcb\.txt/u);
assert.match(plan, /Word 与 WPS/u);
assert.match(docsConfig, /defineConfig/u);
assert.match(docsConfig, /search:\s*\{[\s\S]*provider: "local"[\s\S]*translations:/u);
assert.match(docsConfig, /outline: \{ level: \[2, 3\], label: "本页导航" \}/u);
assert.match(docsConfig, /lastUpdated: \{ text: "最后更新" \}/u);
assert.match(docsConfig, /editLink: \{[\s\S]*text: "编辑此页" \}/u);
assert.match(docsConfig, /srcExclude/u);
assert.doesNotMatch(docsConfig, /ignoreDeadLinks/u, "site links must be checked by VitePress");
assert.match(docsHome, /按任务进入/u);
assert.match(docsHome, /site-footer/u, "documentation home must expose a factual footer navigation");
assert.match(docsHome, /proof-metrics/u, "documentation home must expose current capability evidence");
assert.match(docsConfig, /notFound:\s*\{[\s\S]*title: "页面不存在"[\s\S]*回到文档中心/u, "documentation 404 must expose a localized recovery path");
assert.match(customCss, /\.NotFound/u, "documentation 404 must have product styling");
assert.match(customCss, /\.VPContent \{ overflow-x: clip; \}/u, "documentation content must not clip vertical focus targets");
assert.match(customCss, /html \{ scroll-behavior: auto; \}/u, "documentation must disable smooth scrolling for reduced-motion users");
assert.match(customCss, /@media \(min-width: 761px\) and \(max-width: 1100px\)/u, "documentation hero must keep a medium desktop split layout");
assert.doesNotMatch(customCss, /@media \(max-width: 1100px\) \{ \.hero-grid \{ grid-template-columns: 1fr/u, "documentation hero must not collapse at the sidebar viewport boundary");
assert.match(docsHome, /<b>84\.28%<\/b>/u, "documentation home coverage must match the current validation evidence");
assert.match(validation, /84\.28% Rust line coverage/u, "documentation validation must expose the current coverage evidence");
assert.match(validation, /Frontend: 358 tests\. Statements 88\.75%, branches 83\.51%, functions 91\.23%, lines 92\.50%/u, "frontend coverage evidence must match the latest full run");
assert.equal(packageJson.scripts["docs:dev"], "vitepress dev docs");
assert.equal(packageJson.scripts["docs:build"], "vitepress build docs");
assert.equal(packageJson.scripts["docs:preview"], "vitepress preview docs");
for (const file of ["docs/user-guide.md", "docs/release.md", "docs/validation.md", "docs/safety.md", "docs/competitive-audit.md", "docs/product.md", "docs/security.md", "docs/support-policy.md", "docs/design.md", "docs/changelog.md"]) {
  await read(file);
}

for (const [name, contents] of Object.entries({ README: readme, "README.zh-CN": readmeZh, DESIGN: design, "docs/README": docsIndex, "docs/ARCHITECTURE": architecture, "docs/PLAN": plan })) {
  assert.doesNotMatch(contents, /[A-Z]:\\Users\\/u, `${name} publishes a workstation path`);
}

console.log("Verified product descriptions, version metadata, documentation entry points and external-validation boundaries.");
