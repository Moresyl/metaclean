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
    expect(localStorage.getItem(HISTORY_STORAGE_KEY)).toBeNull();
  });

  it("rejects oversized local history before parsing or rendering it", () => {
    localStorage.setItem(HISTORY_STORAGE_KEY, "{".repeat(2_000_001));
    expect(loadHistory()).toEqual([]);
    expect(localStorage.getItem(HISTORY_STORAGE_KEY)).toBeNull();
  });

  it("rejects a history entry beyond the native batch result limit", () => {
    localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify([{
      ...entry("too-many-results"),
      results: Array.from({ length: 10_001 }, () => entry("one").results[0]),
    }]));
    expect(loadHistory()).toEqual([]);
  });

  it("keeps the newest one hundred entries", () => {
    const entries = Array.from({ length: 105 }, (_, index) => entry(String(index)));
    expect(limitHistory(entries)).toHaveLength(100);
    expect(limitHistory(entries).at(-1)?.id).toBe("99");
  });

  it("keeps the newest results within a bounded history render budget", () => {
    const entries = Array.from({ length: 100 }, (_, index) => ({
      ...entry(String(index)),
      results: Array.from({ length: 200 }, (_, resultIndex) => ({
        ...entry(`${index}-${resultIndex}`).results[0],
        sourcePath: `${index}-${resultIndex}.jpg`,
      })),
    }));
    const limited = limitHistory(entries);
    expect(limited).toHaveLength(50);
    expect(limited.reduce((total, item) => total + item.results.length, 0)).toBe(10_000);
    expect(limited.at(-1)?.results).toHaveLength(200);

    const oversizedNextBatch = limitHistory([
      { ...entry("first"), results: Array.from({ length: 6_000 }, (_, index) => ({ ...entry(String(index)).results[0], sourcePath: `${index}.jpg` })) },
      { ...entry("second"), results: Array.from({ length: 5_000 }, (_, index) => ({ ...entry(`next-${index}`).results[0], sourcePath: `next-${index}.jpg` })) },
    ]);
    expect(oversizedNextBatch.map((item) => item.id)).toEqual(["first"]);
    expect(oversizedNextBatch[0].results).toHaveLength(6_000);
  });

  it("keeps in-memory history when the browser storage quota is exhausted", () => {
    const set = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new DOMException("quota"); });
    expect(persistHistory([entry("kept")])).toEqual([entry("kept")]);
    set.mockRestore();
  });
});
