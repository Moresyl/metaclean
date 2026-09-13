import type { CleanResult, Finding, ScanReport } from "../types";
import {
  MAX_BATCH_FILES,
  MAX_DIAGNOSTIC_BYTES,
  MAX_LABEL_BYTES,
  MAX_NAME_BYTES,
  MAX_PATH_BYTES,
  isBoundedText,
} from "./bounds";

const MAX_FINDINGS = 10_000;
const MAX_TOTAL_FINDINGS = 100_000;

function isSafeSize(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isOptionalSize(value: unknown): value is number | undefined {
  return value === undefined || value === null || isSafeSize(value);
}

function isOptionalPath(value: unknown): value is string | undefined {
  return value === undefined || value === null || isBoundedText(value, MAX_PATH_BYTES);
}

function isOptionalDiagnostic(value: unknown): value is string | undefined {
  return value === undefined || value === null || isBoundedText(value, MAX_DIAGNOSTIC_BYTES, true);
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function isFinding(value: unknown): value is Finding {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<Finding>;
  return isBoundedText(candidate.category, MAX_LABEL_BYTES)
    && isBoundedText(candidate.label, MAX_LABEL_BYTES)
    && isSafeSize(candidate.count)
    && (candidate.severity === "privacy" || candidate.severity === "provenance" || candidate.severity === "informational");
}

function isFindingList(value: unknown): value is Finding[] {
  if (!Array.isArray(value) || value.length > MAX_FINDINGS) return false;
  for (const finding of value) {
    if (!isFinding(finding)) return false;
  }
  return true;
}

function isScanReport(value: unknown): value is ScanReport {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<ScanReport>;
  return isBoundedText(candidate.path, MAX_PATH_BYTES)
    && isBoundedText(candidate.name, MAX_NAME_BYTES)
    && isBoundedText(candidate.format, MAX_LABEL_BYTES)
    && isSafeSize(candidate.size)
    && typeof candidate.supported === "boolean"
    && isFindingList(candidate.findings)
    && isOptionalDiagnostic(candidate.error);
}

function normalizedScanReport(value: ScanReport): ScanReport {
  return {
    path: value.path,
    name: value.name,
    format: value.format,
    size: value.size,
    supported: value.supported,
    findings: value.findings.map((finding) => ({
      category: finding.category,
      label: finding.label,
      count: finding.count,
      severity: finding.severity,
    })),
    error: optionalString(value.error),
  };
}

function isCleanResult(value: unknown): value is CleanResult {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<CleanResult>;
  return isBoundedText(candidate.sourcePath, MAX_PATH_BYTES)
    && isOptionalPath(candidate.outputPath)
    && isOptionalPath(candidate.backupPath)
    && isOptionalSize(candidate.sourceSize)
    && isOptionalSize(candidate.outputSize)
    && isFindingList(candidate.removed)
    && typeof candidate.success === "boolean"
    && isOptionalDiagnostic(candidate.error);
}

function normalizedCleanResult(value: CleanResult): CleanResult {
  return {
    sourcePath: value.sourcePath,
    outputPath: optionalString(value.outputPath),
    backupPath: optionalString(value.backupPath),
    sourceSize: typeof value.sourceSize === "number" ? value.sourceSize : undefined,
    outputSize: typeof value.outputSize === "number" ? value.outputSize : undefined,
    removed: value.removed.map((finding) => ({
      category: finding.category,
      label: finding.label,
      count: finding.count,
      severity: finding.severity,
    })),
    success: value.success,
    error: optionalString(value.error),
  };
}

/** Validate complete native scan responses before they enter React state. */
export function normalizeScanReports(value: unknown): ScanReport[] | undefined {
  if (!Array.isArray(value) || value.length > MAX_BATCH_FILES) return undefined;
  let findings = 0;
  for (const report of value) {
    if (!isScanReport(report)) return undefined;
    findings += report.findings.length;
    if (findings > MAX_TOTAL_FINDINGS) return undefined;
  }
  return value.map(normalizedScanReport);
}

/** Validate complete native cleanup responses before reconciliation/history. */
export function normalizeCleanResults(value: unknown): CleanResult[] | undefined {
  if (!Array.isArray(value) || value.length > MAX_BATCH_FILES) return undefined;
  let findings = 0;
  for (const result of value) {
    if (!isCleanResult(result)) return undefined;
    findings += result.removed.length;
    if (findings > MAX_TOTAL_FINDINGS) return undefined;
  }
  return value.map(normalizedCleanResult);
}
