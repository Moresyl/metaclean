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
      height: 570,
      minWidth: 1100,
      minHeight: 570,
      maxWidth: 1100,
      maxHeight: 570,
      resizable: false,
      maximizable: false,
    },
  );
});
