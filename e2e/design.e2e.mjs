import assert from "node:assert/strict";

function assertResolvedColor(actual, expected, message) {
  if (actual === expected) return;
  // Finished color transitions can retain an Oklab round trip with a few
  // extra decimal places. Keep alpha exact and bound each color component.
  assert.ok(actual.startsWith("oklab(") && expected.startsWith("oklab("), message);
  const components = value => value.match(/[-+]?(?:\d*\.?\d+)(?:e[-+]?\d+)?/giu).map(Number);
  const left = components(actual);
  const right = components(expected);
  assert.equal(left.length, right.length);
  left.forEach((channel, index) => assert.ok(Math.abs(channel - right[index]) <= (index === 3 ? 0 : 0.0001),
    `${message}: ${actual} / ${expected}`));
}

describe("Desktop visual controls", () => {
  it("preserves native select geometry, disabled ink and keyboard focus in both themes", async () => {
    await $(".app-shell").waitForDisplayed();
    const previousTheme = await browser.tauri.execute(() => document.documentElement.dataset.theme);
    try {
      await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "4", ctrlKey: true })));
      await $(".locale-switch select").waitForDisplayed();
      assert.equal(await browser.tauri.execute(() => getComputedStyle(document.querySelector(".locale-switch select")).fontSize), "12px");
      await browser.tauri.execute(() => {
        const host = document.createElement("div");
        host.id = "select-regression";
        host.style.cssText = "position:fixed;top:80px;left:80px;pointer-events:none;z-index:9999";
        host.append(document.querySelector(".locale-switch select").parentElement.cloneNode(true));
        document.body.append(host);
      });
      for (const theme of ["light", "dark"]) {
        await browser.tauri.execute((_, theme) => document.documentElement.dataset.theme = theme, theme);
        for (const mode of ["normal", "disabled", "invalid"]) {
          await browser.tauri.execute((_, mode) => {
            const select = document.querySelector("#select-regression select");
            select.disabled = mode === "disabled";
            select.setAttribute("aria-invalid", String(mode === "invalid"));
          }, mode);
          await browser.waitUntil(async () => browser.tauri.execute(() => !document.querySelector("#select-regression")
            .getAnimations({ subtree: true }).some(animation => animation.pending || animation.playState === "running")));
          const state = await browser.tauri.execute((_, theme, mode) => {
            const select = document.querySelector("#select-regression select");
            const style = getComputedStyle(select);
            const icon = getComputedStyle(select.nextElementSibling);
            const probe = document.createElement("span");
            const ink = theme === "light" ? "#1a1c1f" : "#dfdfdf";
            probe.style.color = mode === "disabled" ? `color-mix(in oklab, ${theme === "light" ? ink : "#ffffff"} 50%, transparent)` : ink;
            const border = mode === "disabled" ? `color-mix(in oklab, ${ink} 6%, transparent)` : mode === "invalid"
              ? theme === "light" ? "#e02e2a" : "#ba2623"
              : theme === "light" ? "color-mix(in oklab, #1a1c1f 12%, transparent)" : "color-mix(in oklab, #ffffff 16%, transparent)";
            probe.style.boxShadow = `inset 0 0 0 1px ${border}`;
            document.body.append(probe);
            const expected = getComputedStyle(probe);
            const result = { height: style.height, radius: style.borderRadius, font: style.fontSize, weight: style.fontWeight,
              lineHeight: style.lineHeight, gutter: style.paddingInlineStart, background: style.backgroundColor,
              color: style.color, expectedColor: expected.color, border: style.boxShadow, expectedBorder: expected.boxShadow,
              cursor: style.cursor, cornerScaling: CSS.supports("corner-shape", "superellipse(1.5)"),
              icon: { width: icon.width, height: icon.height, opacity: icon.opacity, color: icon.color } };
            probe.remove();
            return result;
          }, theme, mode);
          assert.equal(state.height, "32px");
          assert.equal(state.radius, state.cornerScaling ? "10px" : "8px");
          assert.equal(state.font, "12px");
          assert.equal(state.weight, "500");
          assert.equal(state.lineHeight, "24px");
          assert.equal(state.gutter, "12px");
          assert.equal(state.background, "rgba(0, 0, 0, 0)");
          assert.equal(state.cursor, mode === "disabled" ? "not-allowed" : "pointer");
          assertResolvedColor(state.color, state.expectedColor, `${theme}/${mode} select ink`);
          const color = value => value.match(/(?:oklab|color|rgba?)\([^)]*\)/)?.[0];
          assertResolvedColor(color(state.border), color(state.expectedBorder), `${theme}/${mode} select border`);
          assert.equal(state.icon.width, "8px");
          assert.equal(state.icon.height, "12px");
          assert.equal(state.icon.opacity, "0.75");
          if (mode === "disabled") assertResolvedColor(state.icon.color, state.expectedColor, `${theme} disabled select indicator`);
        }
        await browser.keys("Tab");
        for (const invalid of [false, true]) {
          await browser.tauri.execute((_, invalid) => {
            const select = document.querySelector("#select-regression select");
            select.setAttribute("aria-invalid", String(invalid));
            select.focus();
          }, invalid);
          await browser.waitUntil(async () => browser.tauri.execute(() => {
            const select = document.querySelector("#select-regression select");
            return select.matches(":focus-visible") && getComputedStyle(select).outlineWidth === "2px";
          }), { timeoutMsg: `${theme}/${invalid} select focus did not settle` });
          const ring = await browser.tauri.execute((_, theme, invalid) => {
            const style = getComputedStyle(document.querySelector("#select-regression select"));
            const probe = document.createElement("span");
            probe.style.color = invalid ? "#ff8583" : theme === "light" ? "#339cff" : "color-mix(in oklab, #339cff 70%, transparent)";
            document.body.append(probe);
            const result = { color: style.outlineColor, expected: getComputedStyle(probe).color, offset: style.outlineOffset };
            probe.remove();
            return result;
          }, theme, invalid);
          assertResolvedColor(ring.color, ring.expected, `${theme}/${invalid} select ring`);
          assert.equal(ring.offset, "-1px");
        }
      }
    } finally {
      await browser.tauri.execute((_, previousTheme) => {
        document.querySelector("#select-regression")?.remove();
        document.documentElement.dataset.theme = previousTheme;
      }, previousTheme);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("keeps queued files and toolbar actions visible in narrow content", async () => {
    await $(".app-shell").waitForDisplayed();
    await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true })));
    try {
      await browser.tauri.execute(() => {
        document.querySelector("main > .animate-rise").style.inlineSize = "325px";
        const transfer = new DataTransfer();
        transfer.items.add(new File(["Synthetic layout fixture"], "layout-fixture.txt", { type: "text/plain" }));
        const input = document.querySelector("input[type=file]");
        input.files = transfer.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await $(".file-queue .file-item").waitForDisplayed();
      await browser.waitUntil(async () => browser.tauri.execute(() => !document.querySelector("main > .animate-rise")
        .getAnimations({ subtree: true }).some(animation => animation.pending || animation.playState === "running")));
      const state = await browser.tauri.execute(() => {
        const queue = document.querySelector(".file-queue");
        const bounds = queue.getBoundingClientRect();
        const file = queue.querySelector(".file-item").getBoundingClientRect();
        return { height: bounds.height, fileTop: file.top - bounds.top, fileBottom: file.bottom - bounds.top,
          actions: [...queue.querySelectorAll("header button, header select")].map(action => {
            const rect = action.getBoundingClientRect();
            return { left: rect.left - bounds.left, right: rect.right - bounds.left };
          }), width: bounds.width };
      });
      assert.ok(state.height >= 260 && state.height <= 360);
      assert.ok(state.fileTop >= 0 && state.fileBottom <= state.height, "The first queued file must remain visible");
      assert.ok(state.actions.length >= 5);
      for (const action of state.actions) assert.ok(action.left >= 0 && action.right <= state.width + 1,
        "Every queue toolbar action must fit inside the queue");
    } finally {
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("keeps empty intake content contained when the workspace becomes narrow", async () => {
    await $(".app-shell").waitForDisplayed();
    await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true })));
    await $(".drop-zone").waitForDisplayed();
    try {
      await browser.tauri.execute(() => {
        document.querySelector("main > .animate-rise").style.inlineSize = "325px";
      });
      await browser.waitUntil(async () => browser.tauri.execute(() => {
        const region = document.querySelector("main > .animate-rise");
        return Math.abs(region.getBoundingClientRect().width - 325) < 1
          && !region.getAnimations({ subtree: true }).some(animation => animation.pending || animation.playState === "running");
      }));
      const bounds = await browser.tauri.execute(() => {
        const workspace = document.querySelector(".clean-workspace");
        const intake = workspace.querySelector(".drop-zone");
        const card = intake.getBoundingClientRect();
        const options = workspace.querySelector(".clean-options").getBoundingClientRect();
        return { columns: getComputedStyle(workspace).gridTemplateColumns.split(" ").length,
          width: workspace.clientWidth, scrollWidth: workspace.scrollWidth,
          height: card.height, optionsGap: options.top - card.bottom,
          children: [...intake.children].filter(child => child.tagName === "DIV").map(child => {
            const rect = child.getBoundingClientRect();
            return { top: rect.top - card.top, bottom: rect.bottom - card.top, left: rect.left - card.left, right: rect.right - card.left };
          }), cardWidth: card.width };
      });
      assert.equal(bounds.columns, 1, "Narrow content must stack intake and options");
      assert.ok(bounds.scrollWidth <= bounds.width + 1, "Workspace content must not spill horizontally");
      assert.ok(bounds.optionsGap >= 15, "Options must follow the complete intake card");
      for (const child of bounds.children) {
        assert.ok(child.top >= 0 && child.bottom <= bounds.height + 1, "Intake content must stay inside its card");
        assert.ok(child.left >= 0 && child.right <= bounds.cardWidth + 1, "Intake content must fit its width");
      }
    } finally {
      await browser.tauri.execute(() => document.querySelector("main > .animate-rise").style.removeProperty("inline-size"));
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("preserves action geometry, readable disabled states and focus in both themes", async () => {
    await $(".app-shell").waitForDisplayed();
    const previousTheme = await browser.tauri.execute(() => document.documentElement.dataset.theme);
    try {
      await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true })));
      await $(".scan-button").waitForDisplayed();
      assert.equal(await browser.tauri.execute(() => getComputedStyle(document.querySelector(".scan-button")).fontWeight), "400");
      await browser.tauri.execute(() => {
        const host = document.createElement("div");
        host.id = "action-regression";
        host.style.cssText = "position:fixed;top:80px;left:80px;display:flex;flex-wrap:wrap;width:600px;gap:8px;pointer-events:none;z-index:9999";
        for (const variant of ["primary", "secondary", "ghost", "danger"]) {
          for (const size of ["sm", "md", "lg"]) {
            const action = document.querySelector(".scan-button").cloneNode(true);
            action.className = "action";
            action.dataset.variant = variant;
            action.dataset.size = size;
            action.disabled = false;
            host.append(action);
          }
        }
        document.body.append(host);
      });
      for (const theme of ["light", "dark"]) {
        await browser.tauri.execute((_, theme) => document.documentElement.dataset.theme = theme, theme);
        for (const disabled of [false, true]) {
          await browser.tauri.execute((_, disabled) => {
            document.querySelectorAll("#action-regression button").forEach(action => { action.disabled = disabled; });
          }, disabled);
          await browser.waitUntil(async () => browser.tauri.execute((_, theme, disabled) => {
            if (document.querySelector("#action-regression").getAnimations({ subtree: true })
              .some(animation => animation.pending || animation.playState === "running")) return false;
            const action = document.querySelector("#action-regression button");
            const paint = getComputedStyle(action, "::before").backgroundColor;
            if (disabled) {
              const canvas = document.createElement("canvas"); canvas.width = canvas.height = 1;
              const context = canvas.getContext("2d"); context.fillStyle = paint; context.fillRect(0, 0, 1, 1);
              return context.getImageData(0, 0, 1, 1).data[3] === 13;
            }
            return paint === (theme === "light" ? "rgb(26, 28, 31)" : "rgb(223, 223, 223)");
          }, theme, disabled), { timeoutMsg: "Action theme and disabled transitions did not settle" });
          const states = await browser.tauri.execute(() => [...document.querySelectorAll("#action-regression button")].map(action => {
            const style = getComputedStyle(action);
            const surface = getComputedStyle(action, "::before");
            const canvas = document.createElement("canvas"); canvas.width = canvas.height = 1;
            const context = canvas.getContext("2d");
            const paint = value => {
              context.clearRect(0, 0, 1, 1); context.fillStyle = value; context.fillRect(0, 0, 1, 1);
              return [...context.getImageData(0, 0, 1, 1).data];
            };
            const probe = document.createElement("span");
            probe.style.color = document.documentElement.dataset.theme === "light"
              ? "color-mix(in oklab, #1a1c1f 50%, transparent)"
              : "color-mix(in oklab, #ffffff 50%, transparent)";
            document.body.append(probe);
            const disabledColor = getComputedStyle(probe).color;
            probe.remove();
            return { variant: action.dataset.variant, size: action.dataset.size,
              height: style.height, radius: style.borderRadius, font: style.fontSize,
              weight: style.fontWeight, lineHeight: style.lineHeight, gap: style.columnGap,
              gutter: style.paddingLeft, opacity: style.opacity, cursor: style.cursor,
              color: style.color, fill: surface.backgroundColor, surfaceOpacity: surface.opacity,
              fillPaint: paint(surface.backgroundColor),
              disabledColor,
              borderPaint: paint(surface.boxShadow.match(/(?:oklab|color|rgba?)\([^)]*\)/)?.[0] ?? "transparent"),
              transform: style.transform, scale: style.scale };
          }));
          assert.equal(states.length, 12);
          for (const state of states) {
            assert.equal(state.height, { sm: "28px", md: "32px", lg: "36px" }[state.size]);
            assert.equal(state.radius, "9999px");
            assert.equal(state.font, state.size === "md" ? "14px" : "12px");
            assert.equal(state.lineHeight, state.font);
            assert.equal(state.weight, "400");
            assert.equal(state.gap, state.size === "sm" ? "4px" : "6px");
            assert.equal(state.gutter, state.size === "sm" ? "13.3px" : "15.96px");
            assert.equal(state.cursor, disabled ? "not-allowed" : "pointer");
            assert.equal(state.opacity, disabled && state.variant === "ghost" ? "0.4" : "1");
            assert.equal(state.surfaceOpacity, state.variant === "ghost" ? "0" : "1");
            assert.equal(state.transform, "none");
            assert.equal(state.scale, "none");
            if (disabled) {
              assertResolvedColor(state.color, state.disabledColor,
                `${theme}/${state.variant}/${state.size} disabled ink differs from the specified color`);
              if (state.variant === "secondary") {
                assert.equal(state.fill, "rgba(0, 0, 0, 0)");
                assert.equal(state.borderPaint[3], 15);
              } else assert.equal(state.fillPaint[3], 13);
            } else if (state.variant === "primary") {
              assert.equal(state.fill, theme === "light" ? "rgb(26, 28, 31)" : "rgb(223, 223, 223)");
              assert.equal(state.color, theme === "light" ? "rgb(255, 255, 255)" : "rgb(24, 24, 24)");
            } else if (state.variant === "danger") {
              assert.equal(state.fill, "rgb(224, 46, 42)");
              assert.equal(state.color, "rgb(255, 255, 255)");
            } else assert.equal(state.color, theme === "light" ? "rgb(26, 28, 31)" : "rgb(223, 223, 223)");
          }
        }
        await browser.tauri.execute(() => document.querySelectorAll("#action-regression button").forEach(action => { action.disabled = false; }));
        await browser.keys("Tab");
        for (const variant of ["primary", "secondary", "ghost", "danger"]) {
          await browser.tauri.execute((_, variant) => document.querySelector(`#action-regression [data-variant=${variant}]`).focus(), variant);
          await browser.waitUntil(async () => browser.tauri.execute((_, variant) => {
            const action = document.querySelector(`#action-regression [data-variant=${variant}]`);
            return action.matches(":focus-visible") && getComputedStyle(action, "::after").outlineWidth === "2px";
          }, variant), { timeoutMsg: `${theme}/${variant} action focus ring did not settle` });
          const focus = await browser.tauri.execute((_, variant) => {
            const action = document.querySelector(`#action-regression [data-variant=${variant}]`);
            const ring = getComputedStyle(action, "::after");
            const probe = document.createElement("span");
            probe.style.color = variant === "danger" ? "#ff8583"
              : document.documentElement.dataset.theme === "light" ? "#339cff" : "color-mix(in oklab, #339cff 70%, transparent)";
            document.body.append(probe);
            const expected = getComputedStyle(probe).color;
            probe.remove();
            return { visible: action.matches(":focus-visible"), width: ring.outlineWidth, offset: ring.outlineOffset, color: ring.outlineColor, expected };
          }, variant);
          assert.equal(focus.visible, true);
          assert.equal(focus.width, "2px");
          assert.equal(focus.offset, ["primary", "danger"].includes(variant) ? "2px" : "-1px");
          assertResolvedColor(focus.color, focus.expected, `${theme}/${variant} focus color`);
        }
      }
    } finally {
      await browser.tauri.execute((_, previousTheme) => {
        document.querySelector("#action-regression")?.remove();
        document.documentElement.dataset.theme = previousTheme;
      }, previousTheme);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

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
            scaledCorners: CSS.supports("corner-shape", "superellipse(1.5)"),
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
        assert.equal(state.radius, state.scaledCorners ? "5px" : "4px");
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

  it("matches neutral checkbox borders, disabled states and focus in both themes", async () => {
    const previousTheme = await browser.tauri.execute(() => document.documentElement.dataset.theme);
    const colors = {
      light: { border: [205, 205, 205, 255], selected: [24, 24, 24, 255],
        disabledBorder: [223, 223, 223, 255], disabledFill: [252, 252, 252, 255], disabledSelected: [175, 175, 175, 255], ring: [51, 156, 255, 255] },
      dark: { border: [93, 93, 93, 255], selected: [237, 237, 237, 255],
        disabledBorder: [48, 48, 48, 255], disabledFill: [33, 33, 33, 255], disabledSelected: [33, 33, 33, 255], ring: [51, 156, 255, 179] },
    };
    const readStyle = async () => browser.tauri.execute(() => {
      const check = document.querySelector("#checkbox-state-regression");
      const style = getComputedStyle(check);
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 1;
      const context = canvas.getContext("2d");
      const color = value => {
        context.clearRect(0, 0, 1, 1);
        context.fillStyle = value;
        context.fillRect(0, 0, 1, 1);
        return [...context.getImageData(0, 0, 1, 1).data];
      };
      return { border: color(style.borderTopColor), fill: color(style.backgroundColor), ring: color(style.outlineColor),
        width: style.width, height: style.height, radius: style.borderRadius, borderWidth: style.borderTopWidth,
        cursor: style.cursor, outline: style.outlineWidth, offset: style.outlineOffset,
        focus: check.matches(":focus-visible"),
        scaledCorners: CSS.supports("corner-shape", "superellipse(1.5)") };
    });
    const sameColor = (actual, expected) => actual.every((channel, index) => Math.abs(channel - expected[index]) <= 1);
    const expectColors = async (border, fill, message) => {
      await browser.waitUntil(async () => {
        const actual = await readStyle();
        return sameColor(actual.border, border) && sameColor(actual.fill, fill);
      }, { timeoutMsg: message });
    };
    try {
      await browser.tauri.execute(() => {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true }));
      });
      await $(".clean-options .check").waitForDisplayed();
      // Exercise the actual native renderer's stylesheet without changing saved
      // cleanup preferences or React's controlled checkbox state.
      await browser.tauri.execute(() => {
        const fixture = document.querySelector(".clean-options .check").cloneNode();
        fixture.id = "checkbox-state-regression";
        fixture.setAttribute("aria-label", "Checkbox visual regression");
        // The embedded driver's pointer actions dispatch synthetic MouseEvents;
        // they cannot activate CSS :hover. Browser qualification covers hover.
        // Keep the physical cursor from altering these native state samples.
        fixture.style.cssText = "position:fixed;left:100px;top:100px;z-index:9999;pointer-events:none";
        document.body.append(fixture);
      });
      for (const theme of ["light", "dark"]) {
        const palette = colors[theme];
        await browser.tauri.execute((_, theme) => { document.documentElement.dataset.theme = theme; }, theme);
        for (const state of ["unchecked", "checked", "indeterminate", "disabled-unchecked", "disabled-checked", "disabled-indeterminate"]) {
          await browser.tauri.execute((_, state) => {
            const check = document.querySelector("#checkbox-state-regression");
            check.blur();
            check.checked = state.endsWith("checked") && !state.endsWith("unchecked");
            check.indeterminate = state.endsWith("indeterminate");
            check.disabled = state.startsWith("disabled");
          }, state);
          const disabled = state.startsWith("disabled");
          const checked = state.endsWith("checked") && !state.endsWith("unchecked");
          const selected = checked || state.endsWith("indeterminate");
          const border = disabled ? checked ? palette.disabledSelected : palette.disabledBorder : selected ? palette.selected : palette.border;
          const fill = disabled ? checked ? palette.disabledSelected : palette.disabledFill : selected ? palette.selected : [0, 0, 0, 0];
          await expectColors(border, fill, `${theme}/${state} checkbox colors did not settle`);
          const actual = await readStyle();
          assert.equal(actual.width, "18px");
          assert.equal(actual.height, "18px");
          assert.equal(actual.borderWidth, "1px");
          assert.equal(actual.radius, actual.scaledCorners ? "5px" : "4px");
          assert.equal(actual.cursor, disabled ? "not-allowed" : "pointer");
        }
        await browser.tauri.execute(() => {
          const check = document.querySelector("#checkbox-state-regression");
          check.disabled = check.checked = check.indeterminate = false;
          check.focus();
        });
        const focused = await readStyle();
        assert.equal(focused.focus, true);
        assert.equal(focused.outline, "2px");
        assert.equal(focused.offset, "2px");
        assert.ok(sameColor(focused.ring, palette.ring), `${theme} focus ring differs from its theme`);
      }
    } finally {
      await browser.tauri.execute((_, previousTheme) => {
        document.querySelector("#checkbox-state-regression")?.remove();
        if (previousTheme) document.documentElement.dataset.theme = previousTheme;
      }, previousTheme);
    }
  });
});
