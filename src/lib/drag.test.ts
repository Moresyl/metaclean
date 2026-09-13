import { describe, expect, it } from "vitest";
import { normalizeNativeDropEvent } from "./drag";

describe("normalizeNativeDropEvent", () => {
  it("keeps bounded drop paths and event type", () => {
    expect(normalizeNativeDropEvent({ type: "drop", paths: ["C:\\work\\photo.jpg"] })).toEqual({
      type: "drop", paths: ["C:\\work\\photo.jpg"],
    });
  });

  it("drops malformed events and filters invalid paths", () => {
    expect(normalizeNativeDropEvent(null)).toBeUndefined();
    expect(normalizeNativeDropEvent({ type: "unknown", paths: [] })).toBeUndefined();
    expect(normalizeNativeDropEvent({ type: "drop" })).toBeUndefined();
    expect(normalizeNativeDropEvent({ type: "over" })).toEqual({ type: "over", paths: [] });
    expect(normalizeNativeDropEvent({ type: "drop", paths: ["", 4, "界".repeat(20_000), "ok.txt"] })).toEqual({ type: "drop", paths: ["ok.txt"] });
  });

  it("keeps hover lifecycle events closable even with malformed path payloads", () => {
    expect(normalizeNativeDropEvent({ type: "leave", paths: Array.from({ length: 40_001 }, () => "bad") })).toEqual({
      type: "leave", paths: [],
    });
  });

  it("fails closed when a drop path array has been revoked", () => {
    const revoked = Proxy.revocable([], {});
    revoked.revoke();
    expect(normalizeNativeDropEvent({ type: "drop", paths: revoked.proxy })).toBeUndefined();
  });

  it("de-duplicates paths before the unique batch limit", () => {
    const paths = [...Array.from({ length: 10_001 }, () => "C:\\same.txt"), "C:\\kept.txt"];
    expect(normalizeNativeDropEvent({ type: "drop", paths })).toEqual({
      type: "drop",
      paths: ["C:\\same.txt", "C:\\kept.txt"],
    });
  });

  it("fails closed when unique paths exceed the batch limit", () => {
    const paths = Array.from({ length: 10_001 }, (_, index) => `C:\\file-${index}.txt`);
    expect(normalizeNativeDropEvent({ type: "drop", paths })).toBeUndefined();
  });

  it("fails closed when the aggregate path payload exceeds the IPC budget", () => {
    const paths = Array.from({ length: 2_200 }, (_, index) => `${index}-${"x".repeat(32_000)}`);
    expect(normalizeNativeDropEvent({ type: "drop", paths })).toBeUndefined();
  });
});
