import { MAX_BATCH_FILES, MAX_BATCH_PATH_BYTES, MAX_PATH_BYTES, MAX_RAW_BATCH_ITEMS, UTF8_ENCODER } from "./bounds";
import { pathIdentity } from "./files";

export interface NativeDropEvent {
  type: "enter" | "over" | "drop" | "leave";
  paths: string[];
}

/** Validate Tauri drag/drop payloads before they reach queue state or IPC. */
export function normalizeNativeDropEvent(value: unknown): NativeDropEvent | undefined {
  if (!value || typeof value !== "object") return undefined;
  const candidate = value as { type?: unknown; paths?: unknown };
  if (candidate.type !== "enter" && candidate.type !== "over" && candidate.type !== "drop" && candidate.type !== "leave") return undefined;
  // Hover/leave notifications carry no paths that the app consumes. Keeping
  // their lifecycle independent from drop payload validation prevents a bad
  // leave payload from leaving the drop zone visually stuck.
  if (candidate.type !== "drop") return { type: candidate.type, paths: [] };
  if (!Array.isArray(candidate.paths)) return candidate.type === "drop" ? undefined : { type: candidate.type, paths: [] };
  if (candidate.paths.length > MAX_RAW_BATCH_ITEMS) return undefined;
  const paths: string[] = [];
  const seen = new Set<string>();
  let total = 0;
  for (const path of candidate.paths) {
    if (typeof path !== "string") continue;
    const bytes = UTF8_ENCODER.encode(path).byteLength;
    if (!path || bytes > MAX_PATH_BYTES) continue;
    // Match the native and picker contracts: an aggregate overflow rejects
    // the whole drop instead of silently processing only its prefix.
    if (total + bytes > MAX_BATCH_PATH_BYTES) return undefined;
    total += bytes;
    const identity = pathIdentity(path);
    if (seen.has(identity)) continue;
    if (paths.length >= MAX_BATCH_FILES) return undefined;
    seen.add(identity);
    paths.push(path);
  }
  return { type: candidate.type, paths };
}
