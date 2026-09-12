import { describe, expect, it } from "vitest";
import { normalizeBatchProgress } from "./progress";

describe("normalizeBatchProgress", () => {
  it("keeps valid native progress events", () => {
    expect(normalizeBatchProgress({ operation: "clean", batchId: "batch-1", completed: 2, total: 3, failed: 1, cancelled: false })).toEqual({
      operation: "clean", batchId: "batch-1", completed: 2, total: 3, failed: 1, cancelled: false,
    });
  });

  it("rejects malformed counters, identities and event types", () => {
    const valid = { operation: "scan", batchId: "batch-1", completed: 1, total: 2, failed: 0, cancelled: false };
    expect(normalizeBatchProgress({ ...valid, operation: "other" })).toBeUndefined();
    expect(normalizeBatchProgress({ ...valid, batchId: "界".repeat(100) })).toBeUndefined();
    expect(normalizeBatchProgress({ ...valid, completed: 3 })).toBeUndefined();
    expect(normalizeBatchProgress({ ...valid, failed: 2 })).toBeUndefined();
    expect(normalizeBatchProgress({ ...valid, total: 0 })).toBeUndefined();
    expect(normalizeBatchProgress({ ...valid, cancelled: "false" })).toBeUndefined();
    expect(normalizeBatchProgress(null)).toBeUndefined();
  });
});
