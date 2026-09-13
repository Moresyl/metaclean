import { describe, expect, it } from "vitest";
import { normalizeContextMenuStatus } from "./system";

describe("native shell status boundary", () => {
  it("accepts the complete status shape", () => {
    expect(normalizeContextMenuStatus({ available: true, enabled: false, detail: "Enable for supported file types" })).toEqual({
      available: true, enabled: false, detail: "Enable for supported file types",
    });
  });

  it.each([
    undefined,
    { available: "true", enabled: false, detail: "x" },
    { available: false, enabled: true, detail: "x" },
    { available: true, enabled: false, detail: "" },
    { available: true, enabled: false, detail: "界".repeat(4_097) },
  ])("rejects malformed shell status: %s", (value) => {
    expect(normalizeContextMenuStatus(value)).toBeUndefined();
  });
});
