import type { CleanMode } from "../types";
import { readStorage, removeStorage, writeStorage } from "./storage";

/**
 * A crash marker deliberately contains no source or output paths. It only
 * tells the next launch that a file operation may have been interrupted and
 * gives the user enough progress context to decide whether to re-import.
 */
export const ACTIVE_BATCH_STORAGE_KEY = "metaclean.activeBatch";
const MAX_ACTIVE_BATCH_STORAGE_CHARS = 1_024;
const MAX_BATCH_ID_BYTES = 128;
const UTF8_ENCODER = new TextEncoder();

export interface ActiveBatchRecovery {
  batchId: string;
  total: number;
  completed: number;
  mode: CleanMode;
  startedAt: string;
}

function isActiveBatchRecovery(value: unknown): value is ActiveBatchRecovery {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<ActiveBatchRecovery>;
  return typeof candidate.batchId === "string"
    && candidate.batchId.length > 0
    && UTF8_ENCODER.encode(candidate.batchId).byteLength <= MAX_BATCH_ID_BYTES
    && typeof candidate.total === "number"
    && Number.isSafeInteger(candidate.total)
    && candidate.total > 0
    && typeof candidate.completed === "number"
    && Number.isSafeInteger(candidate.completed)
    && candidate.completed >= 0
    && candidate.completed <= candidate.total
    && (candidate.mode === "copy" || candidate.mode === "replace")
    && typeof candidate.startedAt === "string"
    && candidate.startedAt.length > 0
    && candidate.startedAt.length <= 64;
}

export function readActiveBatch(): ActiveBatchRecovery | undefined {
  const raw = readStorage(ACTIVE_BATCH_STORAGE_KEY);
  if (!raw) return undefined;
  if (raw.length > MAX_ACTIVE_BATCH_STORAGE_CHARS) {
    removeStorage(ACTIVE_BATCH_STORAGE_KEY);
    return undefined;
  }
  try {
    const parsed: unknown = JSON.parse(raw);
    if (isActiveBatchRecovery(parsed)) return parsed;
  } catch {
    /* Invalid local state is discarded below. */
  }
  removeStorage(ACTIVE_BATCH_STORAGE_KEY);
  return undefined;
}

export function writeActiveBatch(value: ActiveBatchRecovery): boolean {
  if (!isActiveBatchRecovery(value)) return false;
  const serialized = JSON.stringify(value);
  if (serialized.length > MAX_ACTIVE_BATCH_STORAGE_CHARS) return false;
  return writeStorage(ACTIVE_BATCH_STORAGE_KEY, serialized);
}

export function updateActiveBatchProgress(batchId: string, completed: number): boolean {
  const current = readActiveBatch();
  if (!current || current.batchId !== batchId) return false;
  return writeActiveBatch({ ...current, completed: Math.min(current.total, Math.max(0, Math.trunc(completed))) });
}

export function clearActiveBatch(batchId?: string): boolean {
  if (batchId) {
    const current = readActiveBatch();
    if (current && current.batchId !== batchId) return false;
  }
  return removeStorage(ACTIVE_BATCH_STORAGE_KEY);
}
