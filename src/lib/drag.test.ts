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

  it("de-duplicates paths before the unique batch limit", () => {
    const paths = [...Array.from({ length: 10_001 }, () => "C:\\same.txt"), "C:\\kept.txt"];
    expect(normalizeNativeDropEvent({ type: "drop", paths })).toEqual({
      type: "drop",
      paths: ["C:\\same.txt", "C:\\kept.txt"],
    });
  });
});
