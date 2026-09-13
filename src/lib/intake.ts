import type { IntakeIssue, IntakeResult } from "../types";
import { pathIdentity } from "./files";
import { MAX_BATCH_FILES, MAX_BATCH_PATH_BYTES, MAX_PATH_BYTES, UTF8_ENCODER, isBoundedText } from "./bounds";

const MAX_RAW_PATHS = 10_000;
// One final budget-overflow diagnostic can be emitted after the 50,000-entry
// walker budget is exhausted.
const MAX_SKIPPED_COUNT = 50_001;
const MAX_ISSUES = 100;
const MAX_ISSUE_REASON_BYTES = 8 * 1024;
/** Validate path arrays crossing the native/UI boundary before IPC reuse. */
export function normalizePathList(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) return undefined;
  if (value.length > MAX_RAW_PATHS) return undefined;
  const paths: string[] = [];
  const seen = new Set<string>();
  let totalBytes = 0;
  for (const candidate of value) {
    if (!isBoundedText(candidate, MAX_PATH_BYTES)) return undefined;
    const bytes = UTF8_ENCODER.encode(candidate).byteLength;
    totalBytes += bytes;
    if (totalBytes > MAX_BATCH_PATH_BYTES || paths.length >= MAX_BATCH_FILES) return undefined;
    const identity = pathIdentity(candidate);
    if (seen.has(identity)) continue;
    seen.add(identity);
    paths.push(candidate);
  }
  return paths;
}

function normalizeIssues(value: unknown): IntakeIssue[] | undefined {
  if (!Array.isArray(value) || value.length > MAX_ISSUES) return undefined;
  const issues: IntakeIssue[] = [];
  for (const item of value) {
    if (!item || typeof item !== "object") return undefined;
    const candidate = item as Partial<IntakeIssue>;
    if (!isBoundedText(candidate.path, MAX_PATH_BYTES) || !isBoundedText(candidate.reason, MAX_ISSUE_REASON_BYTES)) return undefined;
    issues.push({ path: candidate.path, reason: candidate.reason });
  }
  return issues;
}

/** Fail closed on malformed native directory-expansion responses. */
export function normalizeIntakeResult(value: unknown): IntakeResult | undefined {
  if (!value || typeof value !== "object") return undefined;
  const candidate = value as Partial<IntakeResult>;
  const files = normalizePathList(candidate.files);
  const issues = normalizeIssues(candidate.issues);
  if (!files || !issues) return undefined;
  if (typeof candidate.skippedCount !== "number" || !Number.isSafeInteger(candidate.skippedCount) || candidate.skippedCount < 0 || candidate.skippedCount > MAX_SKIPPED_COUNT) return undefined;
  if (typeof candidate.limitReached !== "boolean") return undefined;
  return {
    files,
    skippedCount: candidate.skippedCount,
    issues,
    limitReached: candidate.limitReached,
  };
}
