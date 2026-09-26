import assert from "node:assert/strict";
import { copyFile, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const metadata = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
let directory;
let previousStorage;

async function navigate(key) {
  await browser.tauri.execute((_, key) => window.dispatchEvent(new KeyboardEvent("keydown", { key, ctrlKey: true })), key);
}

async function capture(name) {
  await browser.tauri.execute(() => document.activeElement?.blur());
  const dimensions = await browser.tauri.execute(() => ({ width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth > innerWidth }));
  assert.deepEqual(dimensions, { width: 1180, height: 720, overflow: false });
  await browser.saveScreenshot(`./assets/${name}.png`);
}

describe("Localized native documentation", () => {
  before(async () => {
    await $(".app-shell").waitForDisplayed();
    previousStorage = await browser.tauri.execute(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.startsWith("metaclean."))));
    directory = await mkdtemp(join(process.env.METACLEAN_DEMO_ROOT ?? tmpdir(), "MetaClean-demo-"));
    // Deliberately synthetic content: no user documents enter published captures.
    await writeFile(join(directory, "Release notes.txt"), "A small\u200b invisible trace.\n");
    await writeFile(join(directory, "Project brief.md"), "# Project brief\nPrepared\u2060 for sharing.\n");
    await writeFile(join(directory, "Meeting notes.txt"), "Meeting\u200b notes\u2060 for the team.\n");
  });

  after(async () => {
    if (previousStorage) {
      await browser.tauri.execute((_, saved) => {
        for (const key of Object.keys(localStorage)) if (key.startsWith("metaclean.")) localStorage.removeItem(key);
        for (const [key, value] of Object.entries(saved)) localStorage.setItem(key, value);
      }, previousStorage);
    }
    if (directory) {
      const parent = resolve(process.env.METACLEAN_DEMO_ROOT ?? tmpdir());
      assert.ok(resolve(directory).startsWith(join(parent, "MetaClean-demo-")));
      await rm(directory, { recursive: true, force: true });
    }
  });

  for (const locale of ["en", "zh"]) {
    it(`captures ${locale} intake, scan, search, cleanup and settings`, async () => {
      await browser.tauri.execute((_, locale) => {
        localStorage.setItem("metaclean.locale", locale);
        localStorage.setItem("metaclean.theme", "dark");
        localStorage.setItem("metaclean.sidebarCollapsed", "false");
      }, locale);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
      await browser.waitUntil(async () => (await $("body").getText()).includes(`v${metadata.version}`));
      await navigate("1");
      await $(".drop-zone").waitForDisplayed();
      await $(".clean-options button[aria-pressed]").click();
      const fidelity = await $$(".clean-options input[type=checkbox]");
      for (let index = 0; index < fidelity.length; index++) {
        if (await fidelity[index].isSelected() !== (index !== 2)) await fidelity[index].click();
      }
      await capture(`metaclean-home-${locale}`);
      assert.equal(await browser.tauri.execute(() => {
        const settings = document.querySelector(".clean-options .overflow-y-auto");
        return settings.scrollHeight > settings.clientHeight;
      }), false, "default preferences must fit without scrolling");
      if (locale === "zh") await copyFile("assets/metaclean-home-zh.png", "assets/metaclean-screenshot.png");
      const paths = ["Release notes.txt", "Project brief.md", "Meeting notes.txt"].map(name => join(directory, name));
      await browser.tauri.execute(({ core }, paths) => core.invoke("plugin:event|emit_to", {
        target: { kind: "Webview", label: "main" }, event: "tauri://drag-drop", payload: { paths, position: { x: 400, y: 300 } },
      }), paths);
      await browser.waitUntil(async () => (await $$(".file-item")).length === 3);
      await capture(`metaclean-intake-${locale}`);
      await $(".scan-button").click();
      await browser.waitUntil(async () => (await $(".scan-button").getText()).includes(locale === "en" ? "Confirm" : "确认"));
      await capture(`metaclean-scan-${locale}`);
      const search = await $("input[type=search]");
      await search.setValue("release");
      await browser.waitUntil(async () => (await $$(".file-item")).length === 1);
      await capture(`metaclean-search-${locale}`);
      await search.setValue("");
      await browser.waitUntil(async () => (await $$(".file-item")).length === 3);
      await $(".scan-button").click();
      await browser.waitUntil(async () => !(await $(".scan-button").isEnabled()));
      await browser.waitUntil(async () => (await $("body").getText()).includes(locale === "en" ? "3 file(s) cleaned" : "3 个文件清理完成"));
      for (const path of paths) {
        const original = await readFile(path, "utf8");
        assert.match(original, /[\u200b\u2060]/u);
        const extension = path.lastIndexOf(".");
        const cleaned = await readFile(`${path.slice(0, extension)}.cleaned${path.slice(extension)}`, "utf8");
        assert.doesNotMatch(cleaned, /[\u200b\u2060]/u);
      }
      await capture(`metaclean-clean-${locale}`);
      await navigate("4");
      await $(".theme-choices").waitForDisplayed();
      await (await $$(".theme-choices button"))[1].click();
      await capture(`metaclean-settings-${locale}`);
      await navigate("1");
      await $(".file-queue").waitForDisplayed();
      await capture(`metaclean-light-${locale}`);
      for (const path of paths) {
        const extension = path.lastIndexOf(".");
        await rm(`${path.slice(0, extension)}.cleaned${path.slice(extension)}`);
      }
    });
  }
});
