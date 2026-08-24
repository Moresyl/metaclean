import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

/**
 * Counted from the source list rather than written down, so the switcher is
 * checked against what the app actually publishes. A hard-coded number here
 * only ever records how many locales existed the day someone last edited it.
 */
const PUBLISHED_LOCALES = [
  ...readFileSync(new URL("../src/lib/locales.ts", import.meta.url), "utf8")
    .match(/export const LOCALES = \[(.*?)\n\] as const;/su)[1]
    .matchAll(/\{ code: "/gu),
].length;

async function openSettingsPage() {
  const navigation = await $$(".sidebar nav button");
  await navigation[3].click();
  await $(".locale-switch select").waitForDisplayed();
}

async function openCleaningPreferences() {
  const navigation = await $$(".sidebar nav button");
  await navigation[3].click();
  const categories = await $$(".settings-nav button");
  await categories[1].click();
  await $(".fidelity-options").waitForDisplayed();
}

describe("MetaClean desktop application", () => {
  before(async () => {
    const [mainWindow] = await browser.getWindowHandles();
    assert.ok(mainWindow, "the desktop application must expose its main window");
    await browser.switchToWindow(mainWindow);
    const shell = await $(".app-shell");
    await shell.waitForDisplayed();
  });

  it("launches the installed webview and exposes the complete navigation", async () => {
    assert.equal(await $(".brand strong").getText(), "MetaClean");
    assert.equal((await $$(".sidebar nav button")).length, 4);
    assert.equal(await $(".scan-button").isEnabled(), false);
  });

  it("supports keyboard navigation across the desktop shell", async () => {
    await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "4", ctrlKey: true })));
    await $(".locale-switch select").waitForDisplayed();
    await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true })));
    await $(".scan-button").waitForDisplayed();
  });

  it("ships every locale and applies right-to-left layout", async () => {
    await openSettingsPage();

    const locale = await $(".locale-switch select");
    await locale.waitForDisplayed();
    assert.equal((await locale.$$("option")).length, PUBLISHED_LOCALES);
    await browser.tauri.execute(() => {
      const select = document.querySelector(".locale-switch select");
      select.value = "ar";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    await browser.waitUntil(async () => await locale.getValue() === "ar");
    const documentLanguage = await browser.tauri.execute(() => ({
      lang: document.documentElement.lang,
      dir: document.documentElement.dir,
    }));
    assert.deepEqual(documentLanguage, { lang: "ar", dir: "rtl" });
  });

  it("exposes named controls and landmark structure to assistive technology", async () => {
    await browser.tauri.execute(() => localStorage.setItem("metaclean.locale", "en"));
    await browser.refresh();
    await $(".app-shell").waitForDisplayed();
    const accessibility = await browser.tauri.execute(() => ({
      language: document.documentElement.lang,
      mainCount: document.querySelectorAll("main").length,
      navigationName: document.querySelector("nav")?.getAttribute("aria-label"),
      unnamedButtons: [...document.querySelectorAll("button")].filter((button) =>
        !(button.getAttribute("aria-label") || button.textContent?.trim() || button.getAttribute("title")),
      ).length,
      unlabeledInputs: [...document.querySelectorAll("input:not([type='file']), select")].filter((input) =>
        !(input.getAttribute("aria-label") || input.closest("label")),
      ).length,
    }));
    assert.deepEqual(accessibility, {
      language: "en",
      mainCount: 1,
      navigationName: "Main navigation",
      unnamedButtons: 0,
      unlabeledInputs: 0,
    });
  });

  it("persists an explicit theme across a real desktop reload", async () => {
    await openSettingsPage();
    const darkTheme = await $(".theme-choices button:nth-child(3)");
    await darkTheme.click();

    const html = await $("html");
    await browser.waitUntil(async () => await html.getAttribute("data-theme") === "dark");
    await browser.refresh();
    await $(".app-shell").waitForDisplayed();
    assert.equal(await $("html").getAttribute("data-theme"), "dark");
  });

  it("persists ICC and macOS xattr fidelity preferences across a real desktop reload", async () => {
    await browser.tauri.execute(() => localStorage.setItem("metaclean.preserveColorProfile", "true"));
    await browser.tauri.execute(() => localStorage.setItem("metaclean.removeExtendedAttributes", "false"));
    await browser.refresh();
    await $(".app-shell").waitForDisplayed();
    await openCleaningPreferences();
    const fidelity = await $$(".fidelity-options input");
    assert.equal(await fidelity[1].isSelected(), true);
    assert.equal(await fidelity[2].isSelected(), false);
    await fidelity[1].click();
    await fidelity[2].click();
    await browser.waitUntil(async () => await browser.tauri.execute(() => localStorage.getItem("metaclean.preserveColorProfile")) === "false");
    await browser.waitUntil(async () => await browser.tauri.execute(() => localStorage.getItem("metaclean.removeExtendedAttributes")) === "true");

    await browser.refresh();
    await $(".app-shell").waitForDisplayed();
    await openCleaningPreferences();
    const refreshedFidelity = await $$(".fidelity-options input");
    assert.equal(await refreshedFidelity[1].isSelected(), false);
    assert.equal(await refreshedFidelity[2].isSelected(), true);
  });

  it("crosses the Tauri IPC boundary without modifying user files", async () => {
    const reports = await browser.tauri.execute(({ core }) => {
      return core.invoke("scan_files", { paths: [] });
    });

    assert.deepEqual(reports, []);
  });

  it("reports whether this desktop package can self-update", async () => {
    const runtime = await browser.tauri.execute(({ core }) => core.invoke("get_update_runtime"));
    assert.deepEqual(runtime, {
      selfUpdateSupported: process.platform !== "linux",
      portable: false,
    });
  });

  it("fails closed for missing input across scan and cleanup IPC", async () => {
    const missingPath = process.platform === "win32" ? "Z:\\metaclean-missing-input.txt" : "/tmp/metaclean-missing-input.txt";
    const outcome = await browser.tauri.execute(({ core }, path) => Promise.all([
      core.invoke("scan_files", { paths: [path] }),
      core.invoke("clean_files", {
        request: {
          paths: [path],
          mode: "copy",
          preserveTimestamps: true,
          preserveOrientation: true,
          preserveColorProfile: true,
          removeExtendedAttributes: false,
        },
      }),
    ]), missingPath);
    assert.equal(outcome[0][0].supported, false);
    assert.ok(outcome[0][0].error);
    assert.equal(outcome[1][0].success, false);
    assert.ok(outcome[1][0].error);
    assert.equal(outcome[1][0].outputPath, null);
  });
});
