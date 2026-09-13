import { describe, expect, it } from "vitest";
import { normalizeCleanResults, normalizeScanReports } from "./results";

const finding = { category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" as const };

describe("native result boundaries", () => {
  it("accepts complete scan and cleanup responses", () => {
    const scan = { path: "C:\\work\\note.txt", name: "note.txt", format: "Text", size: 4, supported: true, findings: [finding], error: null };
    const clean = { sourcePath: scan.path, outputPath: "C:\\work\\note.cleaned.txt", backupPath: null, sourceSize: 4, outputSize: 3, removed: [finding], success: true, error: null };
    expect(normalizeScanReports([scan])).toEqual([{ ...scan, error: undefined }]);
    expect(normalizeCleanResults([clean])).toEqual([{ ...clean, backupPath: undefined, error: undefined }]);
  });

  it.each([
    undefined,
    { path: "C:\\work\\note.txt", name: "note.txt", format: "Text", size: 4, supported: true, findings: "not-an-array" },
    { path: "C:\\work\\note.txt", name: "note.txt", format: "Text", size: -1, supported: true, findings: [] },
    { path: "C:\\work\\note.txt", name: "note.txt", format: "Text", size: 4, supported: true, findings: [{ ...finding, severity: "unknown" }] },
  ])("rejects malformed scan responses: %s", (value) => {
    expect(normalizeScanReports([value])).toBeUndefined();
  });

  it("rejects malformed cleanup paths, sizes and diagnostics", () => {
    const base = { sourcePath: "C:\\work\\note.txt", removed: [], success: false };
    expect(normalizeCleanResults([{ ...base, sourcePath: "" }])).toBeUndefined();
    expect(normalizeCleanResults([{ ...base, outputSize: Number.MAX_SAFE_INTEGER + 1 }])).toBeUndefined();
    expect(normalizeCleanResults([{ ...base, error: "界".repeat(4_097) }])).toBeUndefined();
  });

  it("accepts null optional fields emitted by serde for absent cleanup values", () => {
    const result = { sourcePath: "C:\\work\\note.txt", outputPath: null, backupPath: null, sourceSize: null, outputSize: null, removed: [], success: false, error: null };
    expect(normalizeCleanResults([result])).toEqual([{ ...result, outputPath: undefined, backupPath: undefined, sourceSize: undefined, outputSize: undefined, error: undefined }]);
  });

  it("rejects an unbounded finding list before rendering it", () => {
    const report = { path: "x", name: "x", format: "Text", size: 1, supported: true, findings: Array.from({ length: 10_001 }, () => finding) };
    expect(normalizeScanReports([report])).toBeUndefined();
  });

  it("rejects oversized native result arrays before reconciliation", () => {
    const report = { path: "x", name: "x", format: "Text", size: 1, supported: true, findings: [] };
    expect(normalizeScanReports(Array.from({ length: 10_001 }, () => report))).toBeUndefined();
    expect(normalizeCleanResults(Array.from({ length: 10_001 }, () => ({ sourcePath: "x", removed: [], success: true })))).toBeUndefined();
  });

  it("strips unknown native fields instead of retaining unbounded payloads", () => {
    const report = { path: "x", name: "x", format: "Text", size: 1, supported: true, findings: [{ ...finding, debug: "untrusted" }], debug: "untrusted" };
    const result = { sourcePath: "x", removed: [], success: true, debug: "untrusted" };
    expect(normalizeScanReports([report])).toEqual([{ path: "x", name: "x", format: "Text", size: 1, supported: true, findings: [finding], error: undefined }]);
    expect(normalizeCleanResults([result])).toEqual([{ sourcePath: "x", outputPath: undefined, backupPath: undefined, sourceSize: undefined, outputSize: undefined, removed: [], success: true, error: undefined }]);
  });
});
