export interface NativeDropEvent {
  type: "enter" | "over" | "drop" | "leave";
  paths: string[];
}

const MAX_PATH_BYTES = 32 * 1024;
const MAX_BATCH_PATH_BYTES = 64 * 1024 * 1024;
const MAX_BATCH_FILES = 10_000;
const UTF8_ENCODER = new TextEncoder();

/** Validate Tauri drag/drop payloads before they reach queue state or IPC. */
export function normalizeNativeDropEvent(value: unknown): NativeDropEvent | undefined {
  if (!value || typeof value !== "object") return undefined;
  const candidate = value as { type?: unknown; paths?: unknown };
  if (candidate.type !== "enter" && candidate.type !== "over" && candidate.type !== "drop" && candidate.type !== "leave") return undefined;
  if (!Array.isArray(candidate.paths)) return candidate.type === "drop" ? undefined : { type: candidate.type, paths: [] };
  const paths: string[] = [];
  let total = 0;
  for (const path of candidate.paths) {
    if (typeof path !== "string") continue;
    const bytes = UTF8_ENCODER.encode(path).byteLength;
    if (!path || bytes > MAX_PATH_BYTES) continue;
    if (total + bytes > MAX_BATCH_PATH_BYTES) break;
    paths.push(path);
    total += bytes;
    if (paths.length === MAX_BATCH_FILES) break;
  }
  return { type: candidate.type, paths };
}
