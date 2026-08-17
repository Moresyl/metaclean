import assert from "node:assert/strict";

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

  it("ships every locale and applies right-to-left layout", async () => {
    const navigation = await $$(".sidebar nav button");
    await navigation[3].click();

    const locale = await $(".locale-switch select");
    await locale.waitForDisplayed();
    assert.equal((await locale.$$("option")).length, 26);
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

  it("persists an explicit theme across a real desktop reload", async () => {
    const darkTheme = await $(".theme-choices button:nth-child(3)");
    await darkTheme.click();

    const html = await $("html");
    await browser.waitUntil(async () => await html.getAttribute("data-theme") === "dark");
    await browser.refresh();
    await $(".app-shell").waitForDisplayed();
    assert.equal(await $("html").getAttribute("data-theme"), "dark");
  });

  it("persists the ICC fidelity preference across a real desktop reload", async () => {
    const navigation = await $$(".sidebar nav button");
    await navigation[3].click();
    const fidelity = await $$(".fidelity-options input");
    assert.equal(await fidelity[1].isSelected(), true);
    await fidelity[1].click();
    await browser.waitUntil(async () => await browser.tauri.execute(() => localStorage.getItem("metaclean.preserveColorProfile")) === "false");

    await browser.refresh();
    await $(".app-shell").waitForDisplayed();
    const refreshedNavigation = await $$(".sidebar nav button");
    await refreshedNavigation[3].click();
    const refreshedFidelity = await $$(".fidelity-options input");
    assert.equal(await refreshedFidelity[1].isSelected(), false);
  });

  it("crosses the Tauri IPC boundary without modifying user files", async () => {
    const reports = await browser.tauri.execute(({ core }) => {
      return core.invoke("scan_files", { paths: [] });
    });

    assert.deepEqual(reports, []);
  });
});
