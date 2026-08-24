import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("keeps application metadata, changelog and release notes on one version", async () => {
  const packageMetadata = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
  const tauriConfig = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  const cargoManifest = await readFile(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
  const changelog = await readFile(new URL("../CHANGELOG.md", import.meta.url), "utf8");
  const releaseNotes = await readFile(new URL(`../release-notes/v${packageMetadata.version}.md`, import.meta.url), "utf8");

  assert.equal(tauriConfig.version, packageMetadata.version);
  assert.match(cargoManifest, new RegExp(`^version = "${packageMetadata.version.replace(/\./gu, "\\.")}"$`, "mu"));
  assert.match(changelog, new RegExp(`^## \\[${packageMetadata.version.replace(/\./gu, "\\.")}\\]`, "mu"));
  assert.match(releaseNotes, new RegExp(`^# MetaClean v${packageMetadata.version.replace(/\./gu, "\\.")}$`, "mu"));
});

test("keeps updater trust in the base config and signing in the release-only config", async () => {
  const tauriConfig = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  const releaseConfig = JSON.parse(await readFile(new URL("../src-tauri/tauri.release.conf.json", import.meta.url), "utf8"));
  const updater = tauriConfig.plugins?.updater;

  assert.match(updater.pubkey, /^[A-Za-z0-9+/]+=*$/u);
  assert.deepEqual(updater.endpoints, ["https://github.com/Moresyl/metaclean/releases/latest/download/latest.json"]);
  assert.equal(updater.windows.installMode, "passive");
  assert.equal(tauriConfig.bundle.createUpdaterArtifacts, undefined);
  assert.equal(releaseConfig.bundle.createUpdaterArtifacts, true);
});

test("keeps Windows release builds on the GUI subsystem", async () => {
  const mainSource = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
  assert.match(mainSource, /^#!\[cfg_attr\(not\(debug_assertions\), windows_subsystem = "windows"\)\]$/mu);
});

/** The interface below the title bar, which is what the layout is designed to. */
const CONTENT_HEIGHT = 570;
/** The caption strip the app draws itself, at the metrics Windows uses. */
const CAPTION_HEIGHT = 32;

test("keeps the desktop window at a fixed size", async () => {
  const tauriConfig = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  const [mainWindow] = tauriConfig.app.windows;

  assert.deepEqual(
    {
      width: mainWindow.width,
      height: mainWindow.height,
      minWidth: mainWindow.minWidth,
      minHeight: mainWindow.minHeight,
      maxWidth: mainWindow.maxWidth,
      maxHeight: mainWindow.maxHeight,
      resizable: mainWindow.resizable,
      maximizable: mainWindow.maximizable,
    },
    {
      width: 1100,
      height: CONTENT_HEIGHT + CAPTION_HEIGHT,
      minWidth: 1100,
      minHeight: CONTENT_HEIGHT + CAPTION_HEIGHT,
      maxWidth: 1100,
      maxHeight: CONTENT_HEIGHT + CAPTION_HEIGHT,
      resizable: false,
      maximizable: false,
    },
  );
});

test("keeps both READMEs on the shipped interface-language count", async () => {
  const locales = await readFile(new URL("../src/lib/locales.ts", import.meta.url), "utf8");
  const declared = locales.match(/export const LOCALES = \[(.*?)\n\] as const;/su)?.[1];
  assert.ok(declared, "LOCALES could not be parsed");
  const count = [...declared.matchAll(/\{ code: "/gu)].length;

  // A language nobody knows we ship is a language nobody switches to, so the
  // published number is checked against the list rather than kept by hand.
  assert.match(await readFile(new URL("../README.md", import.meta.url), "utf8"), new RegExp(`^- ${count} complete interface languages`, "mu"));
  assert.match(await readFile(new URL("../README.zh-CN.md", import.meta.url), "utf8"), new RegExp(`^- ${count} 种完整界面语言`, "mu"));
});

test("draws its own caption, and pays for it in window height", async () => {
  const tauriConfig = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  const [mainWindow] = tauriConfig.app.windows;
  const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");

  // Folding the caption into the client area would cost the interface 32px
  // unless the window grows by exactly that much, so the two move together.
  assert.equal(mainWindow.decorations, false);
  assert.equal(mainWindow.shadow, true);
  assert.match(styles, new RegExp(`grid-template-rows:\\s*${CAPTION_HEIGHT}px minmax\\(0, 1fr\\)`, "u"));
});
