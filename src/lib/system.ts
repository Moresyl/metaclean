import type { ContextMenuStatus } from "../types";
import { MAX_DIAGNOSTIC_BYTES, isBoundedText } from "./bounds";

/** Validate the shell-integration status crossing the native/UI boundary. */
export function normalizeContextMenuStatus(value: unknown): ContextMenuStatus | undefined {
  if (!value || typeof value !== "object") return undefined;
  const candidate = value as Partial<ContextMenuStatus>;
  if (typeof candidate.available !== "boolean" || typeof candidate.enabled !== "boolean") return undefined;
  if (candidate.enabled && !candidate.available) return undefined;
  if (!isBoundedText(candidate.detail, MAX_DIAGNOSTIC_BYTES)) return undefined;
  return { available: candidate.available, enabled: candidate.enabled, detail: candidate.detail };
}
