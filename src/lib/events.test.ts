import { describe, expect, it } from "vitest";
import { normalizeCloseBlocked, normalizeNavigationPage } from "./events";

describe("native event boundaries", () => {
  it("accepts known navigation pages and bounded diagnostics", () => {
    expect(normalizeNavigationPage("settings")).toBe("settings");
    expect(normalizeCloseBlocked("任务正在进行，请等待完成。")).toContain("任务正在进行");
  });

  it.each([undefined, "dashboard", { page: "settings" }])("rejects malformed navigation payloads: %s", (value) => {
    expect(normalizeNavigationPage(value)).toBeUndefined();
  });

  it.each([undefined, { reason: "busy" }, "界".repeat(4_097)])("rejects malformed close-blocked payloads: %s", (value) => {
    expect(normalizeCloseBlocked(value)).toBeUndefined();
  });
});
