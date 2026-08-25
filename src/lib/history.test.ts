import { describe, expect, it, vi } from "vitest";
import type { HistoryEntry } from "../types";
import { HISTORY_STORAGE_KEY, limitHistory, loadHistory, persistHistory } from "./history";

const entry = (id: string): HistoryEntry => ({
  id,
  createdAt: "2026-08-25T00:00:00.000Z",
  mode: "copy",
  results: [{ sourcePath: `${id}.jpg`, removed: [], success: true }],
});

describe("history persistence", () => {
  it("loads only structurally valid history entries", () => {
    localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify([
      entry("valid"),
      { id: "missing-results", createdAt: "now", mode: "copy" },
      null,
    ]));
    expect(loadHistory()).toEqual([entry("valid")]);
  });

  it("rejects invalid nested results, findings, dates, and optional fields", () => {
    const valid = entry("valid");
    localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify([
      valid,
      { ...valid, id: "bad-date", createdAt: "not-a-date" },
      { ...valid, id: "empty", results: [] },
      { ...valid, id: "bad-output", results: [{ ...valid.results[0], outputPath: { path: "x" } }] },
      { ...valid, id: "bad-finding", results: [{ ...valid.results[0], removed: [{ category: "x", label: "x", count: -1, severity: "privacy" }] }] },
    ]));
    expect(loadHistory()).toEqual([valid]);
  });

  it.each(["not json", "{}", "null"])("rejects corrupt history: %s", (value) => {
    localStorage.setItem(HISTORY_STORAGE_KEY, value);
    expect(loadHistory()).toEqual([]);
  });

  it("keeps the newest one hundred entries", () => {
    const entries = Array.from({ length: 105 }, (_, index) => entry(String(index)));
    expect(limitHistory(entries)).toHaveLength(100);
    expect(limitHistory(entries).at(-1)?.id).toBe("99");
  });

  it("keeps in-memory history when the browser storage quota is exhausted", () => {
    const set = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new DOMException("quota"); });
    expect(persistHistory([entry("kept")])).toEqual([entry("kept")]);
    set.mockRestore();
  });
});
