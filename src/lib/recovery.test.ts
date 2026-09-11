import { beforeEach, describe, expect, it } from "vitest";
import {
  ACTIVE_BATCH_STORAGE_KEY,
  clearActiveBatch,
  readActiveBatch,
  updateActiveBatchProgress,
  writeActiveBatch,
} from "./recovery";

describe("interrupted batch recovery marker", () => {
  beforeEach(() => localStorage.clear());

  it("round-trips only non-sensitive batch state and clamps progress", () => {
    expect(writeActiveBatch({ batchId: "batch-1", total: 8, completed: 2, mode: "copy", startedAt: "2026-09-12T10:00:00.000Z" })).toBe(true);
    expect(readActiveBatch()).toEqual({ batchId: "batch-1", total: 8, completed: 2, mode: "copy", startedAt: "2026-09-12T10:00:00.000Z" });
    expect(updateActiveBatchProgress("batch-1", 99)).toBe(true);
    expect(readActiveBatch()?.completed).toBe(8);
    expect(localStorage.getItem(ACTIVE_BATCH_STORAGE_KEY)).not.toContain("C:\\\\");
  });

  it("ignores a foreign batch and removes malformed state", () => {
    writeActiveBatch({ batchId: "batch-1", total: 2, completed: 0, mode: "replace", startedAt: "now" });
    expect(updateActiveBatchProgress("batch-2", 1)).toBe(false);
    expect(readActiveBatch()?.completed).toBe(0);
    localStorage.setItem(ACTIVE_BATCH_STORAGE_KEY, JSON.stringify({ batchId: "", total: 0 }));
    expect(readActiveBatch()).toBeUndefined();
    expect(localStorage.getItem(ACTIVE_BATCH_STORAGE_KEY)).toBeNull();
  });

  it("clears only the matching batch marker", () => {
    writeActiveBatch({ batchId: "batch-1", total: 1, completed: 0, mode: "copy", startedAt: "now" });
    expect(clearActiveBatch("batch-2")).toBe(false);
    expect(readActiveBatch()).toBeDefined();
    expect(clearActiveBatch("batch-1")).toBe(true);
    expect(readActiveBatch()).toBeUndefined();
  });
});
