import assert from "node:assert/strict";

describe("Desktop visual controls", () => {
  it("covers the complete viewport when a confirmation opens from animated content", async () => {
    const previous = await browser.tauri.execute(() => localStorage.getItem("metaclean.history"));
    try {
      await browser.tauri.execute(() => localStorage.setItem("metaclean.history", JSON.stringify([{
        id: "viewport-regression", createdAt: "2026-10-06T00:00:00Z", mode: "copy",
        results: [{sourcePath: "D:/demo/viewport.txt", success: true, removed: []}],
      }])));
      await browser.refresh();
      await $(".sidebar nav button:nth-of-type(2)").waitForDisplayed();
      await $(".sidebar nav button:nth-of-type(2)").click();
      await $(".history-entry").waitForDisplayed();
      await $("main section button").click();
      await $("[role=dialog]").waitForDisplayed();
      const bounds = await browser.tauri.execute(() => {
        const dialog = document.querySelector("[role=dialog]");
        const overlay = dialog.closest("[role=presentation]").getBoundingClientRect();
        const panel = dialog.getBoundingClientRect();
        return { left: overlay.left, top: overlay.top, width: overlay.width, height: overlay.height,
          viewportWidth: innerWidth, viewportHeight: innerHeight,
          panelContained: panel.left >= 0 && panel.top >= 0 && panel.right <= innerWidth && panel.bottom <= innerHeight,
          focused: dialog.contains(document.activeElement) };
      });
      assert.equal(bounds.left, 0);
      assert.equal(bounds.top, 0);
      assert.equal(bounds.width, bounds.viewportWidth);
      assert.equal(bounds.height, bounds.viewportHeight);
      assert.equal(bounds.panelContained, true);
      assert.equal(bounds.focused, true);
      await browser.keys("Escape");
      await $("[role=dialog]").waitForDisplayed({ reverse: true });
      assert.equal(await browser.tauri.execute(() => document.activeElement === document.querySelector("main section button")), true);
    } finally {
      await browser.tauri.execute((_, previous) => {
        if (previous === null) localStorage.removeItem("metaclean.history");
        else localStorage.setItem("metaclean.history", previous);
      }, previous);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("keeps About actions inside a narrow content area in both languages", async () => {
    const previousLocale = await browser.tauri.execute(() => localStorage.getItem("metaclean.locale"));
    try {
      for (const locale of ["en", "zh"]) {
        await browser.tauri.execute((_, locale) => localStorage.setItem("metaclean.locale", locale), locale);
        await browser.refresh();
        await $(".sidebar nav button:nth-of-type(5)").waitForDisplayed();
        await $(".sidebar nav button:nth-of-type(5)").click();
        const content = $("main .animate-rise > section");
        await content.waitForDisplayed();
        // Exercise the available content width without changing the application's
        // fixed native window size or relying on browser-only viewport emulation.
        await browser.tauri.execute(() => {
          document.querySelector("main .animate-rise > section").style.inlineSize = "283px";
        });
        await browser.waitUntil(async () => browser.tauri.execute(() => {
          const region = document.querySelector("main .animate-rise > section");
          const runtime = region.querySelector("p[translate=no]");
          return Math.abs(region.getBoundingClientRect().width - 283) < 1
            && getComputedStyle(region.parentElement).opacity === "1"
            && runtime?.textContent.startsWith("v");
        }), { timeoutMsg: "About content did not settle at the requested width" });
        const state = await browser.tauri.execute(() => {
          const region = document.querySelector("main .animate-rise > section");
          const bounds = region.getBoundingClientRect();
          return {
            clientWidth: region.clientWidth,
            scrollWidth: region.scrollWidth,
            actions: [...region.querySelectorAll("button, a")].map(action => {
              const rect = action.getBoundingClientRect();
              return { left: rect.left - bounds.left, right: rect.right - bounds.left };
            }),
          };
        });
        assert.ok(state.scrollWidth <= state.clientWidth + 1, `About overflow in ${locale}: ${state.scrollWidth}/${state.clientWidth}`);
        assert.ok(state.actions.length >= 8, "Expected update, diagnostic, community and project actions");
        for (const action of state.actions) {
          assert.ok(action.left >= 0 && action.right <= state.clientWidth + 1, `About action is clipped in ${locale}`);
        }
      }
    } finally {
      await browser.tauri.execute((_, previousLocale) => {
        if (previousLocale === null) localStorage.removeItem("metaclean.locale");
        else localStorage.setItem("metaclean.locale", previousLocale);
      }, previousLocale);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("keeps checkbox states, focus and control sizing legible in both themes", async () => {
    await $(".app-shell").waitForDisplayed();
    const previous = await browser.tauri.execute(() => ({
      theme: document.documentElement.dataset.theme,
      locale: localStorage.getItem("metaclean.locale"),
    }));
    try {
      await browser.tauri.execute(() => localStorage.setItem("metaclean.locale", "en"));
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
      for (const theme of ["light", "dark"]) {
        await browser.tauri.execute((_, theme) => {
          document.documentElement.dataset.theme = theme;
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true }));
        }, theme);
        await $(".clean-options .check").waitForDisplayed();
        // Theme changes animate the fill; inspect the final color without
        // disabling the transition or accepting an intermediate shade.
        await browser.waitUntil(async () => browser.tauri.execute((_, theme) => {
          const check = document.querySelector(".clean-options .check");
          const expected = !check.checked ? "rgba(0, 0, 0, 0)"
            : theme === "light" ? "rgb(24, 24, 24)" : "rgb(237, 237, 237)";
          return getComputedStyle(check).backgroundColor === expected;
        }, theme), { timeoutMsg: `Checkbox fill did not settle in the ${theme} theme` });
        const state = await browser.tauri.execute(() => {
          const check = document.querySelector(".clean-options .check");
          const action = document.querySelector(".scan-button");
          const style = getComputedStyle(check);
          check.focus();
          return {
            width: style.width, height: style.height, radius: style.borderRadius,
            background: style.backgroundColor,
            checked: check.checked,
            reduced: matchMedia("(prefers-reduced-motion: reduce)").matches,
            focus: document.activeElement === check,
            actionHeight: getComputedStyle(action).height,
            transition: style.transitionDuration,
            outline: getComputedStyle(check).outlineWidth,
          };
        });
        assert.equal(state.width, "18px");
        assert.equal(state.height, "18px");
        assert.equal(state.radius, "4px");
        assert.equal(state.actionHeight, "36px");
        assert.equal(state.focus, true);
        assert.equal(state.outline, "2px");
        assert.ok(state.transition.split(", ").every(value => Number.parseFloat(value) === (state.reduced ? 0.000001 : 0.15)));
        assert.equal(state.background, !state.checked ? "rgba(0, 0, 0, 0)"
          : theme === "light" ? "rgb(24, 24, 24)" : "rgb(237, 237, 237)");
      }
    } finally {
      await browser.tauri.execute((_, previous) => {
        if (previous.locale === null) localStorage.removeItem("metaclean.locale");
        else localStorage.setItem("metaclean.locale", previous.locale);
        if (previous.theme) document.documentElement.dataset.theme = previous.theme;
      }, previous);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });
});
