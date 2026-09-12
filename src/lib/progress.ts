import type { BatchProgress } from "../types";

const MAX_BATCH_ID_BYTES = 128;
const UTF8_ENCODER = new TextEncoder();

/** Validate native progress events before they can affect visible state. */
export function normalizeBatchProgress(value: unknown): BatchProgress | undefined {
  if (!value || typeof value !== "object") return undefined;
  const candidate = value as Partial<BatchProgress>;
  if (candidate.operation !== "scan" && candidate.operation !== "clean") return undefined;
  if (typeof candidate.batchId !== "string" || candidate.batchId.length === 0 || UTF8_ENCODER.encode(candidate.batchId).byteLength > MAX_BATCH_ID_BYTES) return undefined;
  const { completed, total, failed, cancelled } = candidate;
  if (typeof completed !== "number" || !Number.isSafeInteger(completed) || completed < 0) return undefined;
  if (typeof total !== "number" || !Number.isSafeInteger(total) || total <= 0 || completed > total) return undefined;
  if (typeof failed !== "number" || !Number.isSafeInteger(failed) || failed < 0 || failed > completed) return undefined;
  if (typeof cancelled !== "boolean") return undefined;
  return { operation: candidate.operation, batchId: candidate.batchId, completed, total, failed, cancelled };
}
