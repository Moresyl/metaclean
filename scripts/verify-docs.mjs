import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (file) => readFile(path.join(root, file), "utf8");
const [packageJson, tauriJson, readme, readmeZh, docsIndex, architecture, design, plan, validation, docsConfig, docsHome, customCss, docsLogo] = await Promise.all([
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
  read("docs/public/logo.svg"),
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
assert.match(docsConfig, /base: process\.env\.DOCS_BASE \?\? "\/"/u, "documentation must support a project Pages base path");
assert.match(docsConfig, /logo: "\/logo\.svg"/u);
assert.match(docsLogo, /<svg[^>]+viewBox="0 0 32 32"/u, "navigation logo must exist in VitePress's published public directory");
assert.match(docsConfig, /search:\s*\{[\s\S]*provider: "local"[\s\S]*translations:/u);
assert.match(docsConfig, /outline: \{ level: \[2, 3\], label: "本页导航" \}/u);
assert.match(docsConfig, /lastUpdated: \{ text: "最后更新" \}/u);
assert.match(docsConfig, /editLink: \{[\s\S]*text: "编辑此页" \}/u);
assert.match(docsConfig, /srcExclude/u);
assert.doesNotMatch(docsConfig, /ignoreDeadLinks/u, "site links must be checked by VitePress");
assert.match(docsHome, /按任务进入/u);
const [major, minor] = packageJson.version.split(".");
const sourceLine = `${major}.${minor}.x`;
assert.equal(docsHome.match(new RegExp(sourceLine.replaceAll(".", "\\."), "gu"))?.length, 2, "documentation source line must match the package version");
assert.match(docsHome, /site-footer/u, "documentation home must expose a factual footer navigation");
assert.match(docsHome, /proof-metrics/u, "documentation home must expose current capability evidence");
assert.match(await read("docs/user-guide.md"), /\.\.\/assets\/metaclean-screenshot\.png/u, "user guide must show the current desktop interface");
const screenshot = await readFile(path.join(root, "assets", "metaclean-screenshot.png"));
assert.equal(screenshot.toString("hex", 0, 8), "89504e470d0a1a0a", "documentation screenshot must be a PNG");
assert.equal(screenshot.readUInt32BE(16), 1180, "documentation screenshot must use the desktop window width");
assert.equal(screenshot.readUInt32BE(20), 720, "documentation screenshot must use the desktop window height");
assert.match(docsConfig, /notFound:\s*\{[\s\S]*title: "页面不存在"[\s\S]*回到文档中心/u, "documentation 404 must expose a localized recovery path");
assert.match(customCss, /\.NotFound/u, "documentation 404 must have product styling");
assert.match(customCss, /img\[alt="MetaClean 桌面工作区"\]/u, "current product screenshot must have a stable document treatment");
assert.doesNotMatch(customCss, /gradient|backdrop-filter|clamp\(/u, "documentation must use the same restrained desktop visual language");
assert.doesNotMatch(customCss, /letter-spacing:\s*-/u, "documentation typography must not use negative tracking");
assert.match(customCss, /\.VPContent \{ overflow-x: clip; \}/u, "documentation content must not clip vertical focus targets");
assert.match(customCss, /html \{ scroll-behavior: auto; \}/u, "documentation must disable smooth scrolling for reduced-motion users");
assert.match(customCss, /@media \(min-width: 761px\) and \(max-width: 1100px\)/u, "documentation hero must keep a medium desktop split layout");
assert.doesNotMatch(customCss, /@media \(max-width: 1100px\) \{ \.hero-grid \{ grid-template-columns: 1fr/u, "documentation hero must not collapse at the sidebar viewport boundary");
assert.match(docsHome, /<b>84\.28%<\/b>/u, "documentation home coverage must match the current validation evidence");
assert.match(validation, /84\.28% Rust line coverage/u, "documentation validation must expose the current coverage evidence");
assert.match(validation, /Frontend: 397 tests\. Statements 89\.61%, branches 84\.80%, functions 91\.96%, lines 93\.42%/u, "frontend coverage evidence must match the latest full run");
assert.match(design, /264px persistent workspace panel that collapses to a 64px icon/u, "design reference must document both sidebar states");
assert.match(design, /`Ctrl\/Cmd\+B` collapse control/u, "design reference must document the sidebar shortcut");
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
