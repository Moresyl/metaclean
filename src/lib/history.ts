import type { CleanResult, Finding, HistoryEntry } from "../types";
import { readStorage, writeStorage } from "./storage";

export const HISTORY_STORAGE_KEY = "metaclean.history";
const MAX_HISTORY_ENTRIES = 100;

function isOptionalString(value: unknown): value is string | undefined {
  return value === undefined || typeof value === "string";
}

function isOptionalSize(value: unknown): value is number | undefined {
  return value === undefined || (typeof value === "number" && Number.isSafeInteger(value) && value >= 0);
}

function isFinding(value: unknown): value is Finding {
  if (!value || typeof value !== "object") return false;
  const finding = value as Partial<Finding>;
  return typeof finding.category === "string"
    && typeof finding.label === "string"
    && typeof finding.count === "number"
    && Number.isSafeInteger(finding.count)
    && finding.count >= 0
    && (finding.severity === "privacy" || finding.severity === "provenance" || finding.severity === "informational");
}

function isCleanResult(value: unknown): value is CleanResult {
  if (!value || typeof value !== "object") return false;
  const result = value as Partial<CleanResult>;
  return typeof result.sourcePath === "string"
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
  return typeof entry.id === "string" && entry.id.length > 0
    && typeof entry.createdAt === "string" && Number.isFinite(Date.parse(entry.createdAt))
    && (entry.mode === "copy" || entry.mode === "replace")
    && Array.isArray(entry.results)
    && entry.results.length > 0
    && entry.results.every(isCleanResult);
}

export function limitHistory(entries: HistoryEntry[]): HistoryEntry[] {
  return entries.slice(0, MAX_HISTORY_ENTRIES);
}

export function loadHistory(): HistoryEntry[] {
  const stored = readStorage(HISTORY_STORAGE_KEY);
  if (!stored) return [];
  try {
    const value: unknown = JSON.parse(stored);
    return Array.isArray(value) ? limitHistory(value.filter(isHistoryEntry)) : [];
  } catch {
    return [];
  }
}

export function persistHistory(entries: HistoryEntry[]): HistoryEntry[] {
  const limited = limitHistory(entries);
  writeStorage(HISTORY_STORAGE_KEY, JSON.stringify(limited));
  return limited;
}
