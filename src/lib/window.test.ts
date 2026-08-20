import { describe, expect, it } from "vitest";
import { installZoomLock, isZoomShortcut } from "./window";

describe("window zoom lock", () => {
  it("recognizes browser zoom shortcuts without blocking app navigation", () => {
    expect(isZoomShortcut({ altKey: false, ctrlKey: true, key: "+", metaKey: false })).toBe(true);
    expect(isZoomShortcut({ altKey: false, ctrlKey: false, key: "0", metaKey: true })).toBe(true);
    expect(isZoomShortcut({ altKey: false, ctrlKey: true, key: "4", metaKey: false })).toBe(false);
    expect(isZoomShortcut({ altKey: true, ctrlKey: true, key: "-", metaKey: false })).toBe(false);
  });

  it("prevents keyboard and wheel zoom until disposed", () => {
    const dispose = installZoomLock(window);
    const keydown = new KeyboardEvent("keydown", { cancelable: true, ctrlKey: true, key: "=" });
    const wheel = new WheelEvent("wheel", { cancelable: true, ctrlKey: true });
    window.dispatchEvent(keydown);
    window.dispatchEvent(wheel);
    expect(keydown.defaultPrevented).toBe(true);
    expect(wheel.defaultPrevented).toBe(true);

    dispose();
    const afterDispose = new KeyboardEvent("keydown", { cancelable: true, ctrlKey: true, key: "-" });
    window.dispatchEvent(afterDispose);
    expect(afterDispose.defaultPrevented).toBe(false);
  });
});
