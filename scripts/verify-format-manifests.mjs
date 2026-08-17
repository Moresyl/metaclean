import { readFile } from "node:fs/promises";

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

const [engine, frontend, shell, nsis, wix, readme, readmeZh] = await Promise.all([
  read("src-tauri/src/engine.rs"),
  read("src/lib/files.ts"),
  read("src-tauri/src/shell_integration.rs"),
  read("src-tauri/windows/hooks.nsh"),
  read("src-tauri/windows/fragments/context-menu-cleanup.wxs"),
  read("README.md"),
  read("README.zh-CN.md"),
]);

const rustBlock = engine.match(/pub const SUPPORTED_EXTENSIONS:.*?=\s*&\[(.*?)\];/s)?.[1];
if (!rustBlock) throw new Error("Rust SUPPORTED_EXTENSIONS could not be parsed");
const canonical = quotedValues(rustBlock);
assertUnique("Rust intake list", canonical);
if (canonical.length !== 47) throw new Error(`Expected 47 extensions, found ${canonical.length}`);

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
  throw new Error("MSI cleanup IDs must be unique and sequential from 0 through 46");
}

if (!readme.includes("all 47 supported extensions")) {
  throw new Error("README supported-extension count is stale");
}
if (!readmeZh.includes("全部 47 种受支持扩展名")) {
  throw new Error("Chinese README supported-extension count is stale");
}

console.log("Verified 47 extensions across Rust intake, frontend classification, NSIS and MSI cleanup.");
