import { describe, expect, it } from "vitest";
import { normalizeIntakeResult, normalizePathList } from "./intake";

describe("native intake boundary", () => {
  it("deduplicates bounded paths while preserving the first spelling", () => {
    expect(normalizePathList(["C:\\Work\\Photo.PNG", "c:/work/photo.png", "notes.txt"])).toEqual([
      "C:\\Work\\Photo.PNG",
      "notes.txt",
    ]);
  });

  it.each([
    undefined,
    "C:\\file.txt",
    [""],
    ["界".repeat(16_385)],
    [42],
  ])("rejects malformed path arrays: %s", (value) => {
    expect(normalizePathList(value)).toBeUndefined();
  });

  it("rejects aggregate path payloads beyond the IPC budget", () => {
    expect(normalizePathList(Array.from({ length: 2_200 }, (_, index) => `${index}-${"x".repeat(32_000)}`))).toBeUndefined();
  });

  it("rejects an oversized raw array before spending time normalizing it", () => {
    expect(normalizePathList(Array.from({ length: 40_001 }, () => "same.txt"))).toBeUndefined();
    expect(normalizePathList(Array.from({ length: 10_001 }, (_, index) => `${index}.txt`))).toBeUndefined();
  });

  it("de-duplicates repeated paths before the unique-file limit", () => {
    expect(normalizePathList(Array.from({ length: 10_001 }, () => "same.txt"))).toEqual(["same.txt"]);
  });

  it("normalizes a complete directory expansion response", () => {
    expect(normalizeIntakeResult({
      files: ["C:\\work\\one.txt", "C:/work/ONE.txt"],
      skippedCount: 1,
      issues: [{ path: "C:\\work\\skip.bin", reason: "暂不支持此扩展名" }],
      limitReached: false,
    })).toEqual({
      files: ["C:\\work\\one.txt"],
      skippedCount: 1,
      issues: [{ path: "C:\\work\\skip.bin", reason: "暂不支持此扩展名" }],
      limitReached: false,
    });
  });

  it("fails closed on malformed result counters and issue diagnostics", () => {
    const valid = { files: [], skippedCount: 0, issues: [], limitReached: false };
    expect(normalizeIntakeResult({ ...valid, skippedCount: Number.POSITIVE_INFINITY })).toBeUndefined();
    expect(normalizeIntakeResult({ ...valid, skippedCount: 50_002 })).toBeUndefined();
    expect(normalizeIntakeResult({ ...valid, issues: [{ path: "x", reason: "" }] })).toBeUndefined();
    expect(normalizeIntakeResult({ ...valid, limitReached: "false" })).toBeUndefined();
  });
});
