import { describe, expect, it } from "vitest";
import { normalizeAboutInfo } from "./about";

describe("native About response boundary", () => {
  it("accepts bounded runtime details", () => {
    expect(normalizeAboutInfo({ version: "0.7.1", platform: "windows", arch: "x86_64", appDataDir: "C:\\Data" })).toEqual({
      version: "0.7.1", platform: "windows", arch: "x86_64", appDataDir: "C:\\Data", executableDir: undefined,
    });
  });

  it.each([
    undefined,
    { version: "", platform: "windows", arch: "x64" },
    { version: "0.7.1", platform: "windows", arch: "x64", appDataDir: "界".repeat(16_385) },
  ])("rejects malformed runtime details: %s", (value) => {
    expect(normalizeAboutInfo(value)).toBeUndefined();
  });
});
