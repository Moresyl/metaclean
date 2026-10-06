import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const LOCALE_DEFINITIONS = [...readFileSync(new URL("../src/lib/locales.ts", import.meta.url), "utf8")
  .matchAll(/\{ code: "([^"]+)", nativeName: "[^"]+", htmlLang: "([^"]+)"/gu)]
  .map(([, code, htmlLang]) => ({ code, htmlLang }));

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

async function selectInterfaceLocale(locale) {
  const expectedLanguage = LOCALE_DEFINITIONS.find(({ code }) => code === locale)?.htmlLang;
  assert.ok(expectedLanguage, `Missing published locale ${locale}`);
  const settings = await $(".sidebar nav button:nth-of-type(4)");
  await settings.waitForDisplayed();
  await settings.click();
  await $(".locale-switch select").waitForDisplayed();
  await browser.tauri.execute((_, locale) => {
    const select = document.querySelector(".locale-switch select");
    select.value = locale;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }, locale);
  await browser.waitUntil(async () => browser.tauri.execute((_, locale, language) => document.documentElement.lang === language
    && document.querySelector(".locale-switch select").value === locale, locale, expectedLanguage),
    { timeoutMsg: `Interface locale did not become ${locale}` });
}

describe("Desktop visual controls", () => {
  it("sizes the command panel to the viewport and scrolls keyboard selection into view", async () => {
    await $(".app-shell").waitForDisplayed();
    const previous = await browser.tauri.execute(() => ({ theme: document.documentElement.dataset.theme, dir: document.documentElement.dir }));
    try {
      for (const theme of ["light", "dark"]) {
        for (const dir of ["ltr", "rtl"]) {
          await browser.tauri.execute((_, theme, dir) => {
            document.documentElement.dataset.theme = theme;
            document.documentElement.dir = dir;
            const opener = document.querySelector(".titlebar > button[data-tip]");
            opener.focus();
            opener.click();
          }, theme, dir);
          await $(".palette-layer [role=dialog]").waitForDisplayed();
          await browser.waitUntil(async () => browser.tauri.execute(() => !document.querySelector(".palette-layer")
            .getAnimations({ subtree: true }).some(animation => animation.pending || animation.playState === "running")));
          const geometry = await browser.tauri.execute(() => {
            const panel = document.querySelector(".palette-layer [role=dialog]");
            const list = panel.querySelector("[role=listbox]");
            const bounds = panel.getBoundingClientRect();
            const style = getComputedStyle(list);
            return { left: bounds.left, top: bounds.top, right: bounds.right, bottom: bounds.bottom, width: bounds.width,
              viewportWidth: innerWidth, viewportHeight: innerHeight,
              listMaxHeight: parseFloat(style.maxHeight), overscroll: style.overscrollBehaviorY,
              focused: panel.querySelector("[role=combobox]") === document.activeElement };
          });
          const listMaxHeight = Math.min(440, Math.max(120, geometry.viewportHeight * 0.9 - 64));
          const panelMaxHeight = Math.min(listMaxHeight + 64, geometry.viewportHeight - 32);
          assert.ok(Math.abs(geometry.width - Math.min(520, geometry.viewportWidth * 0.92)) <= 1,
            `${theme}/${dir} command width: ${JSON.stringify(geometry)}`);
          assert.ok(Math.abs(geometry.left - (geometry.viewportWidth - geometry.width) / 2) <= 1);
          assert.ok(Math.abs(geometry.top - Math.max(16, (geometry.viewportHeight - panelMaxHeight) / 2)) <= 1);
          assert.ok(geometry.left >= 0 && geometry.top >= 16 && geometry.right <= geometry.viewportWidth
            && geometry.bottom <= geometry.viewportHeight - 16);
          assert.ok(Math.abs(geometry.listMaxHeight - listMaxHeight) <= 1);
          assert.equal(geometry.overscroll, "contain");
          assert.equal(geometry.focused, true);
          // The embedded driver inserts Home/End's WebDriver private-use code
          // into inputs. Dispatch the DOM key here; browser review covers trusted keys.
          await browser.tauri.execute(() => document.querySelector(".palette-layer [role=combobox]")
            .dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true })));
          let lastSelection;
          await browser.waitUntil(async () => {
            lastSelection = await browser.tauri.execute(() => {
              const list = document.querySelector(".palette-layer [role=listbox]");
              const field = document.querySelector(".palette-layer [role=combobox]");
              const enabled = [...list.querySelectorAll("[role=option]:not(:disabled)")];
              const selected = list.querySelector("[aria-selected=true]");
              const item = selected?.getBoundingClientRect();
              const bounds = list.getBoundingClientRect();
              return { selected: selected?.id, last: enabled.at(-1)?.id, activeDescendant: field.getAttribute("aria-activedescendant"),
                query: field.value, focused: field === document.activeElement, scrollTop: list.scrollTop,
                itemTop: item?.top, itemBottom: item?.bottom, listTop: bounds.top, listBottom: bounds.bottom,
                visible: Boolean(item && item.top >= bounds.top && item.bottom <= bounds.bottom + 1) };
            });
            return lastSelection.selected === lastSelection.last && lastSelection.activeDescendant === lastSelection.selected && lastSelection.visible;
          }, { timeoutMsg: `${theme}/${dir} last enabled command was not scrolled into view` }).catch(error => {
            throw new Error(`${theme}/${dir} command selection: ${JSON.stringify(lastSelection)}`, { cause: error });
          });
          await browser.tauri.execute(() => {
            const list = document.querySelector(".palette-layer [role=listbox]");
            const second = list.querySelectorAll("[role=option]:not(:disabled)")[1];
            second.dispatchEvent(new PointerEvent("pointerover", { bubbles: true }));
          });
          assert.equal(await browser.tauri.execute(() => {
            const list = document.querySelector(".palette-layer [role=listbox]");
            return list.querySelector("[aria-selected=true]") === [...list.querySelectorAll("[role=option]:not(:disabled)")].at(-1);
          }), true, `${theme}/${dir} pointer entry replaced the keyboard selection`);
          await browser.tauri.execute(() => {
            document.querySelector(".palette-layer [role=listbox]").style.maxHeight = "120px";
          });
          await browser.waitUntil(async () => browser.tauri.execute(() => {
            const list = document.querySelector(".palette-layer [role=listbox]");
            const item = list.querySelector("[aria-selected=true]").getBoundingClientRect();
            const bounds = list.getBoundingClientRect();
            return list.clientHeight === 120 && item.top >= bounds.top && item.bottom <= bounds.bottom + 1;
          }), { timeoutMsg: `${theme}/${dir} selection became hidden after the scroll area shrank` });
          await browser.tauri.execute(() => {
            document.querySelector(".palette-layer [role=listbox]").style.maxHeight = "";
          });
          await browser.tauri.execute(() => document.querySelector(".palette-layer [role=combobox]")
            .dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true })));
          await browser.waitUntil(async () => browser.tauri.execute(() => {
            const list = document.querySelector(".palette-layer [role=listbox]");
            const selected = list.querySelector("[aria-selected=true]");
            return selected === list.querySelector("[role=option]:not(:disabled)") && selected.getBoundingClientRect().top >= list.getBoundingClientRect().top;
          }), { timeoutMsg: `${theme}/${dir} first enabled command was not scrolled into view` });
          await browser.keys("Escape");
          await $(".palette-layer").waitForDisplayed({ reverse: true });
          assert.equal(await browser.tauri.execute(() => document.activeElement === document.querySelector(".titlebar > button[data-tip]")), true);
        }
      }
    } finally {
      await browser.tauri.execute((_, previous) => {
        document.documentElement.dataset.theme = previous.theme;
        document.documentElement.dir = previous.dir;
      }, previous);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("styles the native option picker while retaining scalar and listbox semantics", async () => {
    await $(".app-shell").waitForDisplayed();
    const previousTheme = await browser.tauri.execute(() => document.documentElement.dataset.theme);
    try {
      await browser.tauri.execute(() => {
        const host = document.createElement("form");
        host.id = "picker-regression";
        host.style.cssText = "position:fixed;left:450px;top:100px;width:160px";
        const select = document.createElement("select");
        select.className = "field";
        select.name = "mode";
        select.innerHTML = '<option value="copy">Safe copy</option><option value="disabled" disabled>Unavailable</option><option value="replace">Replace with backup</option>';
        host.append(select);
        document.body.append(host);
      });
      for (const theme of ["light", "dark"]) {
        const state = await browser.tauri.execute((_, theme) => {
          document.documentElement.dataset.theme = theme;
          const select = document.querySelector("#picker-regression select");
          const style = getComputedStyle(select);
          const supported = CSS.supports("appearance", "base-select") && CSS.supports("selector(select::picker(select))");
          const picker = supported ? getComputedStyle(select, "::picker(select)") : null;
          const options = [...select.options].map(option => ({ weight: getComputedStyle(option).fontWeight,
            opacity: getComputedStyle(option).opacity, padding: getComputedStyle(option).padding }));
          const result = { supported, appearance: style.appearance, height: style.height,
            font: style.fontSize, value: new FormData(select.form).get("mode"), options,
            picker: picker ? { padding: picker.padding, radius: picker.borderRadius, font: picker.fontSize,
              weight: picker.fontWeight, maxHeight: picker.maxHeight, corner: picker.cornerShape,
              transition: picker.transitionDuration, delay: picker.transitionDelay } : null,
            reduced: matchMedia("(prefers-reduced-motion: reduce)").matches,
            scaled: CSS.supports("corner-shape", "superellipse(1.5)") };
          select.multiple = true;
          result.multipleAppearance = getComputedStyle(select).appearance;
          select.multiple = false;
          select.size = 3;
          result.listboxAppearance = getComputedStyle(select).appearance;
          select.removeAttribute("size");
          return result;
        }, theme);
        assert.equal(state.appearance, state.supported ? "base-select" : "none");
        assert.equal(state.height, "32px");
        assert.equal(state.font, "13px");
        assert.equal(state.value, "copy");
        assert.equal(state.multipleAppearance, "none");
        assert.equal(state.listboxAppearance, "none");
        if (state.supported) {
          assert.equal(state.picker.padding, "4px");
          assert.equal(state.picker.radius, state.scaled ? "20px" : "16px");
          assert.equal(state.picker.corner, "superellipse(1)");
          assert.equal(state.picker.font, "13px");
          assert.equal(state.picker.weight, "430");
          if (state.reduced) {
            assert.ok(state.picker.transition.split(",").every(value => Number.parseFloat(value) === 0));
            assert.ok(state.picker.delay.split(",").every(value => Number.parseFloat(value) === 0));
          }
          assert.deepEqual(state.options.map(option => option.weight), ["600", "430", "430"]);
          assert.equal(state.options[1].opacity, "0.5");
          assert.ok(state.options.every(option => option.padding === "5px 8px"));
        }
      }
    } finally {
      await browser.tauri.execute((_, theme) => {
        document.getElementById("picker-regression")?.remove();
        document.documentElement.dataset.theme = theme;
      }, previousTheme);
    }
  });

  it("preserves tooltip styling, viewport bounds and accessible descriptions in both themes", async () => {
    await $(".app-shell").waitForDisplayed();
    const previous = await browser.tauri.execute(() => ({ theme: document.documentElement.dataset.theme,
      direction: document.documentElement.dir }));
    try {
      await browser.tauri.execute(() => {
        const probe = { phase: "setup", events: [] };
        const record = (type, target, trusted) => {
          probe.events.push({ type, time: performance.now(), phase: probe.phase,
            target: target?.nodeName, id: target?.id, trusted, focused: document.hasFocus() });
          if (probe.events.length > 24) probe.events.shift();
        };
        const capture = event => record(event.type, event.target, event.isTrusted);
        const types = ["focus", "blur", "focusin", "focusout", "resize", "wheel", "pointerover", "pointerout", "pointerdown", "keydown"];
        types.forEach(type => window.addEventListener(type, capture, true));
        const observer = new MutationObserver(mutations => {
          for (const mutation of mutations) {
            for (const [nodes, type] of [[mutation.addedNodes, "tooltip-added"], [mutation.removedNodes, "tooltip-removed"]]) {
              for (const node of nodes) {
                if (node instanceof Element && (node.matches("[role=tooltip]") || node.querySelector("[role=tooltip]"))) record(type, node);
              }
            }
          }
        });
        observer.observe(document.body, { childList: true, subtree: true });
        probe.dispose = () => { observer.disconnect(); types.forEach(type => window.removeEventListener(type, capture, true)); };
        window.__tooltipGeometryProbe = probe;
        const host = document.createElement("button");
        host.id = "tooltip-regression-host";
        host.type = "button";
        host.textContent = "Tip";
        host.dataset.tip = "Synthetic tooltip description ".repeat(4);
        document.body.append(host);
      });
      for (const theme of ["light", "dark"]) {
        for (const direction of ["ltr", "rtl"]) {
          for (const corner of ["top-left", "bottom-right"]) {
            const opening = await browser.tauri.execute((_, theme, direction, corner) => {
              const probe = window.__tooltipGeometryProbe;
              probe.phase = `${theme}/${direction}/${corner}`;
              probe.events = [];
              probe.focusedBefore = document.hasFocus();
              document.documentElement.dataset.theme = theme;
              document.documentElement.dir = direction;
              const host = document.querySelector("#tooltip-regression-host");
              host.style.cssText = `position:fixed;left:${corner === "top-left" ? 2 : innerWidth - 2}px;top:${corner === "top-left" ? 2 : innerHeight - 2}px;width:1px;height:1px;overflow:hidden`;
              host.focus({ preventScroll: true });
              host.dispatchEvent(new PointerEvent("pointerover", { bubbles: true }));
              return { phase: probe.phase, focusedBefore: probe.focusedBefore, focusedAfter: document.hasFocus(),
                hostFocused: document.activeElement === host, events: probe.events };
            }, theme, direction, corner);
            console.log(`Tooltip opening: ${JSON.stringify(opening)}`);
            await $("[role=tooltip]").waitForDisplayed().catch(async error => {
              let diagnostic;
              try {
                diagnostic = await browser.tauri.execute(() => {
                  const tip = document.querySelector("[role=tooltip]");
                  const host = document.querySelector("#tooltip-regression-host");
                  const style = tip ? getComputedStyle(tip) : null;
                  const probe = window.__tooltipGeometryProbe;
                  return { phase: probe.phase, events: probe.events, focused: document.hasFocus(),
                    hostConnected: host?.isConnected, hostFocused: document.activeElement === host,
                    describedBy: host?.getAttribute("aria-describedby"), tip: tip ? { id: tip.id,
                      display: style.display, visibility: style.visibility, opacity: style.opacity,
                      transform: style.transform, bounds: tip.getBoundingClientRect().toJSON(),
                      animations: tip.getAnimations().map(animation => ({ state: animation.playState, pending: animation.pending,
                        time: animation.currentTime, timing: animation.effect?.getComputedTiming() })) } : null };
                });
              } catch (diagnosticError) { diagnostic = { unavailable: diagnosticError.message }; }
              throw new Error(`${theme}/${direction}/${corner} tooltip was not displayed: ${JSON.stringify(diagnostic)}`, { cause: error });
            });
            await browser.waitUntil(async () => browser.tauri.execute(() => !document.querySelector("[role=tooltip]")
              .getAnimations().some(animation => animation.pending || animation.playState === "running")));
            const state = await browser.tauri.execute(() => {
              const tip = document.querySelector("[role=tooltip]");
              const host = document.querySelector("#tooltip-regression-host");
              const style = getComputedStyle(tip);
              const bounds = tip.getBoundingClientRect();
              const canvas = document.createElement("canvas");
              canvas.width = canvas.height = 1;
              const context = canvas.getContext("2d");
              context.fillStyle = style.backgroundColor;
              context.fillRect(0, 0, 1, 1);
              return { probe: { phase: window.__tooltipGeometryProbe.phase, events: window.__tooltipGeometryProbe.events },
                rootMounted: tip.parentElement === document.body, linked: host.getAttribute("aria-describedby") === tip.id,
                id: tip.id, focused: document.activeElement === host, font: style.fontSize, weight: style.fontWeight,
                line: parseFloat(style.lineHeight), padding: style.padding, radius: style.borderRadius, border: style.borderTopWidth,
                scaled: CSS.supports("corner-shape", "superellipse(1.5)"), background: [...context.getImageData(0, 0, 1, 1).data],
                contained: bounds.left >= 15 && bounds.top >= 15 && bounds.right <= innerWidth - 15 && bounds.bottom <= innerHeight - 15,
                width: bounds.width, viewport: innerWidth };
            });
            console.log(`Tooltip state: ${JSON.stringify(state.probe)}`);
            assert.equal(state.rootMounted, true);
            assert.equal(state.linked, true);
            assert.notEqual(state.id, "");
            assert.equal(state.focused, true);
            assert.equal(state.font, "14px");
            assert.equal(state.weight, "400");
            assert.ok(Math.abs(state.line - 20.3) <= 0.1);
            assert.equal(state.padding, "12px 16px");
            assert.equal(state.radius, state.scaled ? "10px" : "8px");
            assert.equal(state.border, "0px");
            const expectedBackground = theme === "light" ? [255, 255, 255, 179] : [33, 33, 33, 245];
            state.background.forEach((channel, index) => assert.ok(Math.abs(channel - expectedBackground[index]) <= 1,
              `${theme} tooltip background: ${JSON.stringify(state.background)}`));
            assert.equal(state.contained, true, `${theme}/${direction}/${corner} tooltip leaves the viewport`);
            assert.ok(state.width <= Math.min(300, state.viewport - 30) + 1);
            await browser.keys("Escape");
            await $("[role=tooltip]").waitForDisplayed({ reverse: true });
            assert.equal(await browser.tauri.execute(() => document.querySelector("#tooltip-regression-host")
              .hasAttribute("aria-describedby")), false);
          }
        }
      }
    } finally {
      await browser.tauri.execute((_, previous) => {
        window.__tooltipGeometryProbe?.dispose();
        delete window.__tooltipGeometryProbe;
        document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        document.getElementById("tooltip-regression-host")?.remove();
        document.documentElement.dataset.theme = previous.theme;
        document.documentElement.dir = previous.direction;
      }, previous);
    }
  });

  it("keeps context menus inside the viewport without scrolling their animated workspace", async () => {
    await $(".app-shell").waitForDisplayed();
    const previousMenuState = await browser.tauri.execute(() => ({ direction: document.documentElement.dir,
      theme: document.documentElement.dataset.theme }));
    try {
      await browser.tauri.execute(() => {
        const probe = { phase: "setup", events: [] };
        const record = (type, target) => {
          probe.events.push({ type, time: performance.now(), phase: probe.phase,
            target: target?.nodeName, role: target?.getAttribute?.("role"), focused: document.hasFocus() });
          if (probe.events.length > 20) probe.events.shift();
        };
        const capture = event => record(event.type, event.target);
        const types = ["focus", "blur", "resize", "wheel", "pointerdown", "contextmenu", "keydown"];
        types.forEach(type => window.addEventListener(type, capture, true));
        const observer = new MutationObserver(mutations => {
          for (const mutation of mutations) {
            for (const [nodes, type] of [[mutation.addedNodes, "menu-added"], [mutation.removedNodes, "menu-removed"]]) {
              for (const node of nodes) {
                if (node instanceof Element && (node.matches(".menu-layer") || node.querySelector(".menu-layer"))) record(type, node);
              }
            }
          }
        });
        observer.observe(document.body, { childList: true, subtree: true });
        probe.dispose = () => { observer.disconnect(); types.forEach(type => window.removeEventListener(type, capture, true)); };
        window.__menuGeometryProbe = probe;
      });
      await $(".sidebar nav button:nth-of-type(1)").click();
      await browser.tauri.execute(() => {
        const input = document.querySelector("input[type=file]");
        const transfer = new DataTransfer();
        transfer.items.add(new File(["Synthetic menu geometry"], "menu-geometry.txt", { type: "text/plain" }));
        input.files = transfer.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await $(".file-queue .file-item").waitForDisplayed();
      await browser.waitUntil(async () => browser.tauri.execute(() => !document.querySelector(".workspace-content")
        .getAnimations({ subtree: true }).some(animation => animation.pending || animation.playState === "running")));
      for (const theme of ["light", "dark"]) {
        for (const { direction, corner } of ["ltr", "rtl"].flatMap(direction =>
          ["top-left", "bottom-right"].map(corner => ({ direction, corner })))) {
          const before = await browser.tauri.execute((_, theme, direction, corner) => {
            window.__menuGeometryProbe.phase = `${theme}/${direction}/${corner}`;
            window.__menuGeometryProbe.events = [];
            window.__menuGeometryProbe.focusedBefore = document.hasFocus();
            document.documentElement.dataset.theme = theme;
            document.documentElement.dir = direction;
            document.querySelector(".queue-search-input").focus({ preventScroll: true });
            const main = document.querySelector("main");
            const scroll = { left: main.scrollLeft, top: main.scrollTop };
            document.querySelector(".file-queue .file-item").dispatchEvent(new MouseEvent("contextmenu",
              { bubbles: true, cancelable: true, clientX: corner === "top-left" ? 2 : innerWidth - 2,
                clientY: corner === "top-left" ? 2 : innerHeight - 2, button: 2 }));
            return scroll;
          }, theme, direction, corner);
          await $("[role=menu]").waitForDisplayed().catch(async error => {
            let diagnostic;
            try {
              diagnostic = await browser.tauri.execute(() => {
                const menu = document.querySelector("[role=menu]");
                const style = menu && getComputedStyle(menu);
                return { phase: window.__menuGeometryProbe?.phase, events: window.__menuGeometryProbe?.events,
                  focused: document.hasFocus(), visibility: document.visibilityState,
                  theme: document.documentElement.dataset.theme, direction: document.documentElement.dir,
                  active: { tag: document.activeElement?.tagName, role: document.activeElement?.getAttribute("role") },
                  menu: menu ? { display: style.display, visibility: style.visibility, opacity: style.opacity,
                    transform: style.transform, bounds: menu.getBoundingClientRect().toJSON(),
                    animations: menu.getAnimations().map(animation => ({ state: animation.playState, pending: animation.pending,
                      time: animation.currentTime, timing: animation.effect?.getComputedTiming() })) } : null };
              });
            } catch (diagnosticError) { diagnostic = { unavailable: diagnosticError.message }; }
            throw new Error(`${theme}/${direction}/${corner} menu was not displayed: ${JSON.stringify(diagnostic)}`, { cause: error });
          });
          await browser.waitUntil(async () => browser.tauri.execute(() => !document.querySelector("[role=menu]")
            .getAnimations({ subtree: true }).some(animation => animation.pending || animation.playState === "running")));
          const state = await browser.tauri.execute(() => {
            const layer = document.querySelector(".menu-layer");
            const overlay = layer.getBoundingClientRect();
            const menu = layer.querySelector("[role=menu]");
            const panel = menu.getBoundingClientRect();
            const main = document.querySelector("main");
            const command = menu.querySelector("[role=menuitem]");
            const previousDisabled = command.disabled;
            command.disabled = true;
            const disabledOpacity = getComputedStyle(command).opacity;
            command.disabled = previousDisabled;
            return { probe: { phase: window.__menuGeometryProbe.phase, focusedBefore: window.__menuGeometryProbe.focusedBefore,
                events: window.__menuGeometryProbe.events },
              layer: { left: overlay.left, top: overlay.top, width: overlay.width, height: overlay.height },
              viewport: { width: innerWidth, height: innerHeight }, rootMounted: layer.parentElement === document.body,
              panel: { left: panel.left, top: panel.top, right: panel.right, bottom: panel.bottom },
              contained: panel.left >= 6 && panel.top >= 6 && panel.right <= innerWidth - 6 && panel.bottom <= innerHeight - 6,
              minimum: panel.width >= Math.min(180, innerWidth - 12), disabledOpacity,
              commandCursors: [...menu.querySelectorAll("[role=menuitem]")].map(item => getComputedStyle(item).cursor),
              focused: document.activeElement === menu, scroll: { left: main.scrollLeft, top: main.scrollTop } };
          });
          console.log(`Context-menu state: ${JSON.stringify(state.probe)}`);
          assert.deepEqual(state.layer, { left: 0, top: 0, width: state.viewport.width, height: state.viewport.height });
          assert.equal(state.rootMounted, true);
          assert.equal(state.contained, true, `${direction}/${corner} menu leaves the viewport: ${JSON.stringify(state.panel)}`);
          if (corner === "top-left") {
            assert.equal(state.panel.left, 6);
            assert.equal(state.panel.top, 6);
          }
          assert.equal(state.minimum, true);
          assert.equal(state.disabledOpacity, "0.5");
          assert.ok(state.commandCursors.every(cursor => cursor === "default"));
          assert.equal(state.focused, true);
          assert.deepEqual(state.scroll, before, `${direction}/${corner} menu scrolls its workspace`);
          for (const key of ["End", "Home"]) {
            // The embedded driver omits WebDriver Home/End mappings. Deliver
            // the actual keys to the native renderer's existing handlers.
            await browser.tauri.execute((_, key) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown",
              { key, code: key, bubbles: true, cancelable: true })), key);
            const selection = await browser.tauri.execute((_, key) => {
              const menu = document.querySelector("[role=menu]");
              const enabled = [...menu.querySelectorAll("[role=menuitem]:not(:disabled)")];
              return { active: menu.getAttribute("aria-activedescendant"),
                expected: (key === "Home" ? enabled[0] : enabled.at(-1)).id,
                singleEntry: [...menu.querySelectorAll("[role=menuitem]")].every(item => item.tabIndex === -1) };
            }, key);
            assert.equal(selection.active, selection.expected, `${key} does not reach the enabled menu boundary`);
            assert.equal(selection.singleEntry, true);
          }
          await browser.keys("Escape");
          await $("[role=menu]").waitForDisplayed({ reverse: true });
          const returned = await browser.tauri.execute(() => ({
            focused: document.activeElement === document.querySelector(".queue-search-input"),
            scroll: { left: document.querySelector("main").scrollLeft, top: document.querySelector("main").scrollTop },
          }));
          assert.equal(returned.focused, true, "Menu dismissal loses the previous queue-search focus");
          assert.deepEqual(returned.scroll, before, "Returning menu focus scrolls the workspace");
        }
      }
    } finally {
      await browser.tauri.execute((_, state) => {
        window.__menuGeometryProbe?.dispose();
        delete window.__menuGeometryProbe;
        document.documentElement.dir = state.direction;
        document.documentElement.dataset.theme = state.theme;
      }, previousMenuState);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("aligns settings segments and keeps the selected thumb visible after selection and resizing", async () => {
    await $(".app-shell").waitForDisplayed();
    const previousTheme = await browser.tauri.execute(() => document.documentElement.dataset.theme);
    const previousDirection = await browser.tauri.execute(() => document.documentElement.dir);
    try {
      await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "4", ctrlKey: true })));
      await $(".settings-nav").waitForDisplayed();
      assert.equal(await browser.tauri.execute(() => getComputedStyle(document.querySelector(".settings-nav .segmented-control")
        ?? document.querySelector(".settings-nav")).height), "32px");
      for (const theme of ["light", "dark"]) {
        await browser.tauri.execute((_, theme) => { document.documentElement.dataset.theme = theme; }, theme);
        for (const index of [1, 4, 2, 1]) {
          await $(`.settings-nav button:nth-of-type(${index})`).click();
          await browser.waitUntil(async () => browser.tauri.execute(() => {
            const track = document.querySelector(".settings-nav .segmented-control");
            const selected = track.querySelector("button[aria-pressed='true']").getBoundingClientRect();
            const thumb = track.querySelector(".segment-thumb").getBoundingClientRect();
            return Math.abs(thumb.left - selected.left) < 0.6 && Math.abs(thumb.width - selected.width) < 0.6;
          }), { timeoutMsg: `${theme}/${index} segment thumb did not align` });
        }
        const state = await browser.tauri.execute(() => {
          const track = document.querySelector(".settings-nav .segmented-control");
          const option = track.querySelector("button");
          const thumb = track.querySelector(".segment-thumb");
          const rootStyle = getComputedStyle(track);
          const itemStyle = getComputedStyle(option);
          return { height: rootStyle.height, gap: rootStyle.columnGap, padding: rootStyle.paddingTop,
            radius: rootStyle.borderRadius, corner: rootStyle.cornerShape,
            font: rootStyle.fontSize, weight: rootStyle.fontWeight, surface: rootStyle.backgroundColor,
            itemHeight: itemStyle.height, itemRadius: itemStyle.borderRadius, gutter: itemStyle.paddingInlineStart,
            line: itemStyle.lineHeight, thumbColor: getComputedStyle(thumb).backgroundColor,
            shadow: getComputedStyle(thumb).boxShadow, scaled: CSS.supports("corner-shape", "superellipse(1.5)") };
        });
        assert.equal(state.height, "32px");
        assert.equal(state.gap, "2px");
        assert.equal(state.padding, "2px");
        assert.equal(state.radius, state.scaled ? "10px" : "8px");
        assert.equal(state.font, "13px");
        assert.equal(state.weight, "600");
        assert.equal(state.itemHeight, "28px");
        assert.equal(state.itemRadius, state.scaled ? "8px" : "6px");
        assert.equal(state.gutter, "12px");
        assert.equal(state.line, "13px");
        assert.equal(state.surface, theme === "light" ? "rgb(237, 237, 237)" : "rgb(13, 13, 13)");
        assert.equal(state.thumbColor, theme === "light" ? "rgb(255, 255, 255)" : "rgb(48, 48, 48)");
        assert.equal(state.shadow, "rgba(0, 0, 0, 0.2) 0px 1px 4px -1px");
        const inactive = await browser.tauri.execute((_, theme) => {
          const probe = document.createElement("span");
          probe.style.color = `color-mix(in srgb, ${theme === "light" ? "#1a1c1f" : "#dfdfdf"} 65%, transparent)`;
          document.body.append(probe);
          const actual = getComputedStyle(document.querySelectorAll(".settings-nav button")[1]).color;
          const expected = getComputedStyle(probe).color;
          probe.remove();
          return { actual, expected };
        }, theme);
        assert.equal(inactive.actual, inactive.expected);
        await browser.tauri.execute(() => document.querySelectorAll(".settings-nav button")[1].focus());
        await browser.waitUntil(async () => browser.tauri.execute(() => {
          const button = document.querySelectorAll(".settings-nav button")[1];
          return button.matches(":focus-visible") && getComputedStyle(button).color === getComputedStyle(document.querySelector(".settings-nav button")).color;
        }), { timeoutMsg: `${theme} focused segment ink did not settle` });
        assert.equal(await browser.tauri.execute(() => getComputedStyle(document.querySelectorAll(".settings-nav button")[1]).outlineWidth), "2px");
        await browser.tauri.execute(() => { document.querySelector("main > .animate-rise").style.inlineSize = "220px"; });
        await $(".settings-nav button:nth-of-type(4)").click();
        await browser.waitUntil(async () => browser.tauri.execute(() => {
          const track = document.querySelector(".settings-nav .segmented-control");
          const bounds = track.getBoundingClientRect();
          const selected = track.querySelector("button[aria-pressed='true']").getBoundingClientRect();
          const thumb = track.querySelector(".segment-thumb").getBoundingClientRect();
          return selected.left >= bounds.left && selected.right <= bounds.right + 0.6
            && Math.abs(thumb.left - selected.left) < 0.6 && Math.abs(thumb.width - selected.width) < 0.6;
        }), { timeoutMsg: `${theme} narrow selection was clipped` });
        await browser.tauri.execute(() => { document.documentElement.dir = "rtl"; });
        await $(".settings-nav button:nth-of-type(2)").click();
        await browser.waitUntil(async () => browser.tauri.execute(() => {
          const track = document.querySelector(".settings-nav .segmented-control");
          const bounds = track.getBoundingClientRect();
          const selected = track.querySelector("button[aria-pressed='true']").getBoundingClientRect();
          const thumb = track.querySelector(".segment-thumb").getBoundingClientRect();
          return selected.left >= bounds.left && selected.right <= bounds.right + 0.6
            && Math.abs(thumb.left - selected.left) < 0.6 && Math.abs(thumb.width - selected.width) < 0.6;
        }), { timeoutMsg: `${theme} RTL segment thumb did not align` });
        await browser.tauri.execute((_, previousDirection) => { document.documentElement.dir = previousDirection; }, previousDirection);
        await browser.tauri.execute(() => { document.querySelector("main > .animate-rise").style.inlineSize = ""; });
        await $(".settings-nav button:nth-of-type(1)").click();
        assert.equal(await browser.tauri.execute(() => getComputedStyle(document.querySelector(".theme-choices")).height), "32px");
      }
    } finally {
      await browser.tauri.execute((_, previousTheme, previousDirection) => {
        document.documentElement.dataset.theme = previousTheme;
        document.documentElement.dir = previousDirection;
        document.querySelector("main > .animate-rise").style.inlineSize = "";
        document.querySelector(".settings-nav button")?.click();
      }, previousTheme, previousDirection);
    }
  });

  it("preserves queue search geometry and input states in both themes", async () => {
    await $(".app-shell").waitForDisplayed();
    const previousTheme = await browser.tauri.execute(() => document.documentElement.dataset.theme);
    try {
      await browser.tauri.execute(() => {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true }));
        const transfer = new DataTransfer();
        transfer.items.add(new File(["Synthetic search fixture"], "search-fixture.txt", { type: "text/plain" }));
        const input = document.querySelector("input[type=file]");
        input.files = transfer.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await $(".file-queue input[type=search]").waitForDisplayed();
      const initial = await browser.tauri.execute(() => ({ radius: getComputedStyle(document.querySelector(".file-queue input[type=search]").parentElement).borderRadius,
        cornerScaling: CSS.supports("corner-shape", "superellipse(1.5)") }));
      assert.equal(initial.radius, initial.cornerScaling ? "10px" : "8px");
      for (const theme of ["light", "dark"]) {
        await browser.tauri.execute((_, theme) => document.documentElement.dataset.theme = theme, theme);
        for (const mode of ["normal", "focus", "readonly", "disabled", "invalid"]) {
          await browser.tauri.execute((_, mode) => {
            const input = document.querySelector(".file-queue input[type=search]");
            input.disabled = mode === "disabled";
            input.readOnly = mode === "readonly";
            input.setAttribute("aria-invalid", String(mode === "invalid"));
            if (["focus", "readonly", "invalid"].includes(mode)) input.focus(); else input.blur();
          }, mode);
          await browser.waitUntil(async () => {
            const sample = await browser.tauri.execute((_, theme, mode) => {
              const input = document.querySelector(".file-queue input[type=search]");
              const container = input.parentElement;
              const focused = ["focus", "readonly", "invalid"].includes(mode);
              if ((document.activeElement === input) !== focused
                || container.matches(":focus-within") !== focused) return false;
              // A renderer can expose the previous computed value before it
              // registers the new transition. Require the actual final surface
              // as well as completed animations before taking the snapshot.
              const probe = document.createElement("span");
              probe.style.backgroundColor = theme === "light" ? "color-mix(in oklab, #1a1c1f 2%, transparent)" : "color-mix(in oklab, #ffffff 3%, transparent)";
              if (focused) probe.style.boxShadow = `inset 0 0 0 1px ${mode === "invalid"
                ? theme === "light" ? "#e02e2a" : "#ba2623"
                : `color-mix(in oklab, ${theme === "light" ? "#1a1c1f" : "#dfdfdf"} 20%, transparent)`}`;
              document.body.append(probe);
              const expected = getComputedStyle(probe);
              const actual = getComputedStyle(container);
              const settled = { border: actual.boxShadow, expectedBorder: expected.boxShadow,
                background: actual.backgroundColor, expectedBackground: expected.backgroundColor,
                running: container.getAnimations({ subtree: true }).some(animation => animation.pending || animation.playState === "running") };
              probe.remove();
              return settled;
            }, theme, mode);
            if (!sample || sample.running) return false;
            try {
              assertResolvedColor(sample.background, sample.expectedBackground, "Search surface settlement");
              if (sample.expectedBorder === "none") return sample.border === "none";
              const color = value => value.match(/(?:oklab|color|rgba?)\([^)]*\)/)?.[0];
              assertResolvedColor(color(sample.border), color(sample.expectedBorder), "Search border settlement");
              return true;
            } catch {
              return false;
            }
          }, { timeoutMsg: `${theme}/${mode} search focus and final surface did not settle` });
          const state = await browser.tauri.execute((_, theme, mode) => {
            const input = document.querySelector(".file-queue input[type=search]");
            const style = getComputedStyle(input);
            const shell = getComputedStyle(input.parentElement);
            const icon = getComputedStyle(input.previousElementSibling);
            const probe = document.createElement("span");
            probe.style.backgroundColor = theme === "light" ? "color-mix(in oklab, #1a1c1f 2%, transparent)" : "color-mix(in oklab, #ffffff 3%, transparent)";
            if (["focus", "readonly", "invalid"].includes(mode)) probe.style.boxShadow = `inset 0 0 0 1px ${mode === "invalid"
              ? theme === "light" ? "#e02e2a" : "#ba2623"
              : `color-mix(in oklab, ${theme === "light" ? "#1a1c1f" : "#dfdfdf"} 20%, transparent)`}`;
            document.body.append(probe);
            const expected = getComputedStyle(probe);
            const result = { height: style.height, font: style.fontSize, weight: style.fontWeight, line: style.lineHeight,
              leading: style.paddingInlineStart, trailing: style.paddingInlineEnd, gutter: shell.paddingInlineStart,
              gap: shell.columnGap, opacity: shell.opacity, cursor: shell.cursor, iconOffset: icon.marginInlineStart,
              background: shell.backgroundColor, expectedBackground: expected.backgroundColor,
              border: shell.boxShadow, expectedBorder: expected.boxShadow,
              focused: document.activeElement === input, inputOutline: style.outlineWidth };
            probe.remove();
            return result;
          }, theme, mode);
          assert.equal(state.height, "32px");
          assert.equal(state.font, "13px");
          assert.equal(state.weight, "400");
          assert.equal(state.line, "19.5px");
          assert.equal(state.leading, "0px");
          assert.equal(state.trailing, "12px");
          assert.equal(state.gutter, "12px");
          assert.equal(state.gap, "8px");
          assert.equal(state.iconOffset, "-2px");
          assert.equal(state.opacity, mode === "disabled" ? "0.5" : "1");
          assert.equal(state.cursor, mode === "disabled" ? "not-allowed" : "text");
          assertResolvedColor(state.background, state.expectedBackground, `${theme}/${mode} search surface`);
          if (state.expectedBorder === "none") assert.equal(state.border, "none");
          else {
            const color = value => value.match(/(?:oklab|color|rgba?)\([^)]*\)/)?.[0];
            assertResolvedColor(color(state.border), color(state.expectedBorder), `${theme}/${mode} search border`);
          }
          assert.equal(state.focused, ["focus", "readonly", "invalid"].includes(mode));
          if (state.focused) assert.equal(state.inputOutline, "0px");
        }
      }
    } finally {
      await browser.tauri.execute((_, previousTheme) => document.documentElement.dataset.theme = previousTheme, previousTheme);
      await browser.refresh();
      await $(".app-shell").waitForDisplayed();
    }
  });

  it("preserves native select geometry, disabled ink and keyboard focus in both themes", async () => {
    await $(".app-shell").waitForDisplayed();
    const previousTheme = await browser.tauri.execute(() => document.documentElement.dataset.theme);
    try {
      await browser.tauri.execute(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "4", ctrlKey: true })));
      await $(".locale-switch select").waitForDisplayed();
      assert.equal(await browser.tauri.execute(() => getComputedStyle(document.querySelector(".locale-switch select")).fontSize), "13px");
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
          assert.equal(state.font, "13px");
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
            assert.equal(state.font, state.size === "md" ? "14px" : "13px");
            assert.equal(state.lineHeight, state.font);
            assert.equal(state.weight, "400");
            assert.equal(state.gap, state.size === "sm" ? "4px" : "6px");
            assert.equal(state.gutter, state.size === "sm" ? "13.3px" : "15.96px");
            assert.equal(state.cursor, disabled ? "not-allowed" : "default");
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
    const previous = await browser.tauri.execute(() => ({ language: document.documentElement.lang, stored: localStorage.getItem("metaclean.locale") }));
    const previousLocale = LOCALE_DEFINITIONS.find(({ htmlLang }) => htmlLang === previous.language)?.code;
    assert.ok(previousLocale, `Missing published language ${previous.language}`);
    try {
      for (const locale of ["en", "zh"]) {
        console.log(`About layout: select ${locale}`);
        await selectInterfaceLocale(locale);
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
        console.log(`About layout: verify ${locale}`);
        assert.ok(state.scrollWidth <= state.clientWidth + 1, `About overflow in ${locale}: ${state.scrollWidth}/${state.clientWidth}`);
        assert.ok(state.actions.length >= 8, "Expected update, diagnostic, community and project actions");
        for (const action of state.actions) {
          assert.ok(action.left >= 0 && action.right <= state.clientWidth + 1, `About action is clipped in ${locale}`);
        }
      }
    } finally {
      await selectInterfaceLocale(previousLocale);
      await browser.tauri.execute((_, stored) => {
        if (stored === null) localStorage.removeItem("metaclean.locale");
        else localStorage.setItem("metaclean.locale", stored);
      }, previous.stored);
      await $(".sidebar nav button:nth-of-type(1)").click();
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
        assert.ok(state.transition.split(", ").every(value => Number.parseFloat(value) === (state.reduced ? 0 : 0.15)));
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
      const root = getComputedStyle(document.documentElement);
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
        borderCss: style.borderTopColor, fillCss: style.backgroundColor,
        theme: document.documentElement.dataset.theme,
        checked: check.checked, indeterminate: check.indeterminate, disabled: check.disabled,
        selectors: { checked: check.matches(":checked"), indeterminate: check.matches(":indeterminate"), disabled: check.matches(":disabled") },
        tokens: Object.fromEntries(["--color-check-border", "--color-check-disabled-border", "--color-check-disabled-selected", "--color-control-active"]
          .map(name => [name, { root: root.getPropertyValue(name), control: style.getPropertyValue(name) }])),
        rendering: { visibility: document.visibilityState, focused: document.hasFocus(), colorScheme: root.colorScheme,
          reduced: matchMedia("(prefers-reduced-motion: reduce)").matches,
          transitionDuration: style.transitionDuration, transitionDelay: style.transitionDelay },
        transitions: check.getAnimations().map(animation => ({ pending: animation.pending, state: animation.playState,
          property: animation.transitionProperty ?? null,
          currentTime: animation.currentTime, timelineTime: animation.timeline?.currentTime,
          timing: animation.effect?.getComputedTiming(), keyframes: animation.effect?.getKeyframes() })),
        width: style.width, height: style.height, radius: style.borderRadius, borderWidth: style.borderTopWidth,
        cursor: style.cursor, outline: style.outlineWidth, offset: style.outlineOffset,
        focus: check.matches(":focus-visible"),
        scaledCorners: CSS.supports("corner-shape", "superellipse(1.5)") };
    });
    const sameColor = (actual, expected) => actual.every((channel, index) => Math.abs(channel - expected[index]) <= 1);
    const expectColors = async (border, fill, message) => {
      let actual;
      try {
        await browser.waitUntil(async () => {
          actual = await readStyle();
          return sameColor(actual.border, border) && sameColor(actual.fill, fill);
        }, { timeoutMsg: message });
      } catch (error) {
        throw new Error(`${message}: ${JSON.stringify({ expected: { border, fill }, actual })}`, { cause: error });
      }
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
          if (actual.rendering.reduced) {
            assert.ok(actual.rendering.transitionDuration.split(",").every(value => Number.parseFloat(value) === 0));
            assert.ok(actual.rendering.transitionDelay.split(",").every(value => Number.parseFloat(value) === 0));
            assert.equal(actual.transitions.filter(animation => animation.property !== null).length, 0);
          }
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
