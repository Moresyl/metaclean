import type { CleanResult, Finding, HistoryEntry } from "../types";
import { readStorage, removeStorage, writeStorage } from "./storage";

export const HISTORY_STORAGE_KEY = "metaclean.history";
const MAX_HISTORY_ENTRIES = 100;
const MAX_HISTORY_RESULTS_PER_ENTRY = 10_000;
const MAX_HISTORY_RESULTS_TOTAL = 10_000;
const MAX_HISTORY_STORAGE_CHARS = 2_000_000;
const MAX_HISTORY_PATH_CHARS = 32_768;
const MAX_HISTORY_LABEL_CHARS = 256;

function isOptionalString(value: unknown, maxLength = MAX_HISTORY_PATH_CHARS): value is string | undefined {
  return value === undefined || (typeof value === "string" && value.length <= maxLength);
}

function isOptionalSize(value: unknown): value is number | undefined {
  return value === undefined || (typeof value === "number" && Number.isSafeInteger(value) && value >= 0);
}

function isFinding(value: unknown): value is Finding {
  if (!value || typeof value !== "object") return false;
  const finding = value as Partial<Finding>;
  return typeof finding.category === "string"
    && finding.category.length <= MAX_HISTORY_LABEL_CHARS
    && typeof finding.label === "string"
    && finding.label.length <= MAX_HISTORY_LABEL_CHARS
    && typeof finding.count === "number"
    && Number.isSafeInteger(finding.count)
    && finding.count >= 0
    && (finding.severity === "privacy" || finding.severity === "provenance" || finding.severity === "informational");
}

function isCleanResult(value: unknown): value is CleanResult {
  if (!value || typeof value !== "object") return false;
  const result = value as Partial<CleanResult>;
  return typeof result.sourcePath === "string"
    && result.sourcePath.length <= MAX_HISTORY_PATH_CHARS
    && typeof result.success === "boolean"
    && isOptionalString(result.outputPath)
    && isOptionalString(result.backupPath)
    && isOptionalString(result.error)
    && isOptionalSize(result.sourceSize)
    && isOptionalSize(result.outputSize)
    && Array.isArray(result.removed)
    && result.removed.every(isFinding);
}

function isHistoryEntry(value: unknown): value is HistoryEntry {
  if (!value || typeof value !== "object") return false;
  const entry = value as Partial<HistoryEntry>;
  const sourcePaths = new Set<string>();
  return typeof entry.id === "string" && entry.id.length > 0 && entry.id.length <= 128
    && typeof entry.createdAt === "string" && entry.createdAt.length <= 64 && Number.isFinite(Date.parse(entry.createdAt))
    && (entry.mode === "copy" || entry.mode === "replace")
    && Array.isArray(entry.results)
    && entry.results.length <= MAX_HISTORY_RESULTS_PER_ENTRY
    && entry.results.length > 0
    && entry.results.every((result) => {
      if (!isCleanResult(result) || sourcePaths.has(result.sourcePath)) return false;
      sourcePaths.add(result.sourcePath);
      return true;
    });
}

export function limitHistory(entries: HistoryEntry[]): HistoryEntry[] {
  let remainingResults = MAX_HISTORY_RESULTS_TOTAL;
  const limited: HistoryEntry[] = [];
  for (const entry of entries.slice(0, MAX_HISTORY_ENTRIES)) {
    if (!entry.results.length || entry.results.length > remainingResults) break;
    limited.push(entry);
    remainingResults -= entry.results.length;
  }
  return limited;
}

export function loadHistory(): HistoryEntry[] {
  const stored = readStorage(HISTORY_STORAGE_KEY);
  if (!stored) return [];
  if (stored.length > MAX_HISTORY_STORAGE_CHARS) {
    removeStorage(HISTORY_STORAGE_KEY);
    return [];
  }
  try {
    const value: unknown = JSON.parse(stored);
    if (!Array.isArray(value)) {
      removeStorage(HISTORY_STORAGE_KEY);
      return [];
    }
    return limitHistory(value.filter(isHistoryEntry));
  } catch {
    removeStorage(HISTORY_STORAGE_KEY);
    return [];
  }
}

export function persistHistory(entries: HistoryEntry[]): HistoryEntry[] {
  const limited = limitHistory(entries);
  writeStorage(HISTORY_STORAGE_KEY, JSON.stringify(limited));
  return limited;
}
