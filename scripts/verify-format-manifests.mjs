import { readFile } from "node:fs/promises";

/**
 * The intake list the whole product is measured against. Hard-coded on purpose:
 * every other list here is checked against the Rust one, so only a fixed number
 * can catch the Rust list itself losing an entry.
 */
const EXPECTED = 91;

const root = new URL("../", import.meta.url);
const read = (path) => readFile(new URL(path, root), "utf8");

function quotedValues(source) {
  return [...source.matchAll(/"([a-z0-9]+)"/g)].map((match) => match[1]);
}

function assertUnique(name, values) {
  const duplicates = values.filter((value, index) => values.indexOf(value) !== index);
  if (duplicates.length > 0) {
    throw new Error(`${name} contains duplicate extensions: ${[...new Set(duplicates)].join(", ")}`);
  }
}

function assertSame(name, expected, actual) {
  assertUnique(name, actual);
  const expectedSorted = [...expected].sort();
  const actualSorted = [...actual].sort();
  if (JSON.stringify(actualSorted) !== JSON.stringify(expectedSorted)) {
    const missing = expectedSorted.filter((value) => !actualSorted.includes(value));
    const extra = actualSorted.filter((value) => !expectedSorted.includes(value));
    throw new Error(`${name} differs from the Rust intake list; missing=[${missing}] extra=[${extra}]`);
  }
}

const [engine, frontend, shell, nsis, wix, readme, readmeZh, policy] = await Promise.all([
  read("src-tauri/src/engine.rs"),
  read("src/lib/files.ts"),
  read("src-tauri/src/shell_integration.rs"),
  read("src-tauri/windows/hooks.nsh"),
  read("src-tauri/windows/fragments/context-menu-cleanup.wxs"),
  read("README.md"),
  read("README.zh-CN.md"),
  read("SUPPORT_POLICY.md"),
]);

const rustBlock = engine.match(/pub const SUPPORTED_EXTENSIONS:.*?=\s*&\[(.*?)\];/s)?.[1];
if (!rustBlock) throw new Error("Rust SUPPORTED_EXTENSIONS could not be parsed");
const canonical = quotedValues(rustBlock);
assertUnique("Rust intake list", canonical);
if (canonical.length !== EXPECTED) throw new Error(`Expected ${EXPECTED} extensions, found ${canonical.length}`);

const frontendBlocks = [...frontend.matchAll(/const \w+_EXTENSIONS = new Set\(\[(.*?)\]\);/gs)];
if (frontendBlocks.length !== 5) throw new Error("Expected five frontend extension groups");
const frontendExtensions = frontendBlocks.flatMap((match) => quotedValues(match[1]));
if (!frontend.includes('extension === "pdf"')) throw new Error("Frontend PDF classification is missing");
frontendExtensions.push("pdf");
assertSame("Frontend classification", canonical, frontendExtensions);

if (!shell.includes("use crate::engine::SUPPORTED_EXTENSIONS;")) {
  throw new Error("Windows shell integration is not using the canonical Rust intake list");
}

const registryPattern = /SystemFileAssociations\\\.([a-z0-9]+)\\shell\\MetaClean/g;
const nsisExtensions = [...nsis.matchAll(registryPattern)].map((match) => match[1]);
const wixExtensions = [...wix.matchAll(registryPattern)].map((match) => match[1]);
assertSame("NSIS uninstall cleanup", canonical, nsisExtensions);
assertSame("MSI uninstall cleanup", canonical, wixExtensions);

const wixIds = [...wix.matchAll(/Id="RemoveMetaCleanContext(\d+)"/g)].map((match) => Number(match[1]));
const expectedIds = Array.from({ length: canonical.length }, (_, index) => index);
if (JSON.stringify(wixIds) !== JSON.stringify(expectedIds)) {
  throw new Error(`MSI cleanup IDs must be unique and sequential from 0 through ${canonical.length - 1}`);
}

if (!readme.includes(`all ${EXPECTED} supported extensions`)) {
  throw new Error("README supported-extension count is stale");
}
if (!readmeZh.includes(`全部 ${EXPECTED} 种受支持扩展名`)) {
  throw new Error("Chinese README supported-extension count is stale");
}

/**
 * The published format table, read back out of the prose that ships it. A count
 * elsewhere in the file can stay right while the table itself quietly describes
 * a version of the product that no longer exists, which is how these two
 * documents once came to promise 47 extensions against an engine shipping 91.
 * The out-of-scope note is the stop mark, because its examples are backticked
 * extensions the engine deliberately does not accept.
 */
function tableExtensions(name, source, heading, closing) {
  const from = source.indexOf(heading);
  const to = source.indexOf(closing, from);
  if (from < 0 || to < 0) throw new Error(`${name} format table could not be located`);
  return [...source.slice(from, to).matchAll(/`\.([a-z0-9]+)`/g)].map((match) => match[1]);
}

assertSame("README format table", canonical, tableExtensions("README", readme, "## What it removes", "**Deliberately out of scope:**"));
assertSame("Chinese README format table", canonical, tableExtensions("Chinese README", readmeZh, "## 清理范围", "**明确不做的事：**"));

if (!policy.includes(`allowlist contains ${EXPECTED} extensions`)) {
  throw new Error("Support policy extension count is stale");
}

// The policy is normative, so both halves of it are held to the engine: every
// family it describes as cleaned has to be accepted, and everything it names as
// refused has to actually be absent from intake.
const described = tableExtensions("Support policy", policy, "| Family | Extensions |", "## Still refused");
assertUnique("Support policy format table", described);
const undescribed = described.filter((value) => !canonical.includes(value));
if (undescribed.length > 0) {
  throw new Error(`Support policy documents extensions the engine does not accept: ${undescribed.join(", ")}`);
}

const refused = [...policy.slice(policy.indexOf("## Still refused")).matchAll(/`\.([a-z0-9]+)`/g)].map((match) => match[1]);
const contradicted = refused.filter((value) => canonical.includes(value));
if (contradicted.length > 0) {
  throw new Error(`Support policy refuses extensions the engine accepts: ${contradicted.join(", ")}`);
}

console.log(`Verified ${EXPECTED} extensions across Rust intake, frontend classification, NSIS and MSI cleanup, both published format tables and the support policy.`);
