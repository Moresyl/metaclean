import type { FileEntry, Finding, ScanReport } from "../types";
import { MAX_BATCH_FILES, MAX_NAME_BYTES, MAX_PATH_BYTES, isBoundedText } from "./bounds";

/* These mirror the engine's intake list. They only choose the glyph on a queue
   row — the format itself is settled by the file's own signature during the
   scan, never by its name. */
const IMAGE_EXTENSIONS = new Set([
  "jpg", "jpeg", "jpe", "png", "webp", "jxl", "gif", "bmp", "dib", "tif", "tiff",
  "heic", "heif", "heics", "heifs", "hif", "avif", "avifs",
  // Raw negatives: TIFF containers under a private magic word, plus Canon's
  // CR3, which is an ISO base media file instead.
  "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "rwl",
  "dng", "pef", "srw", "raf", "3fr", "erf", "mef", "mos", "iiq", "kdc", "dcr", "k25",
]);
const AUDIO_EXTENSIONS = new Set(["mp3", "wav", "flac", "aif", "aiff", "aifc", "wma", "m4a", "f4a", "f4b", "m4b", "m4p", "mka"]);
const VIDEO_EXTENSIONS = new Set([
  "mp4", "mov", "m4v", "3g2", "3gp", "3gp2", "3gpp", "f4p", "f4v", "lrv", "mqv", "qt",
  "avi", "asf", "wmv", "mkv", "mks", "mk3d", "webm",
]);
const DOCUMENT_EXTENSIONS = new Set([
  "docx", "xlsx", "pptx", "odt", "ods", "odp", "odg", "odf", "odb", "odm", "ott", "ots", "otp", "otg", "epub",
]);
const TEXT_EXTENSIONS = new Set([
  "txt", "md", "markdown", "html", "htm", "xhtml", "svg", "xml", "json", "csv", "tsv",
  "yaml", "yml", "log", "srt", "vtt", "css", "scss", "less", "ini", "conf", "cfg", "toml", "properties",
]);

function isWindowsRuntime(): boolean {
  if (typeof navigator === "undefined") return false;
  return /win(?:dows|32|64)/iu.test(`${navigator.platform} ${navigator.userAgent}`);
}

function browserPathIdentity(path: string): string {
  if (!isWindowsRuntime()) return path;
  return path.replaceAll("/", "\\").toLocaleLowerCase("en-US");
}

/**
 * Match the native Windows path identity used by the Rust IPC boundary while
 * keeping POSIX paths case-sensitive. The displayed path remains untouched;
 * this key is only for queue de-duplication and result reconciliation.
 */
export function pathIdentity(path: string): string {
  const normalized = path.replaceAll("/", "\\");
  const lower = normalized.toLocaleLowerCase("en-US");
  const windowsPath = (path.includes("\\") && !path.startsWith("/"))
    || /^[a-z]:[\\/]/iu.test(path)
    || path.startsWith("\\\\")
    || (path.startsWith("//") && isWindowsRuntime());
  if (!windowsPath) return path;
  const withoutDevicePrefix = lower.startsWith("\\\\?\\") ? lower.slice(4) : lower;
  const canonical = withoutDevicePrefix.startsWith("unc\\")
    ? `\\\\${withoutDevicePrefix.slice(4)}`
    : withoutDevicePrefix;
  const absolute = canonical.startsWith("\\\\") || /^[a-z]:\\/u.test(canonical);
  const minimum = canonical.startsWith("\\\\") ? 2 : absolute ? 1 : 0;
  const parts: string[] = [];
  for (const part of canonical.replace(/^[\\]+/u, "").split("\\")) {
    if (!part || part === ".") continue;
    if (part === ".." && parts.length > minimum) {
      parts.pop();
    } else if (part === ".." && !absolute) {
      parts.push(part);
    } else if (part !== "..") {
      parts.push(part);
    }
  }
  let result = parts.join("\\");
  if (canonical.startsWith("\\\\")) result = `\\\\${result}`;
  if (absolute && /^[a-z]:$/u.test(result)) result += "\\";
  return result;
}

export function classifyFile(name: string): FileEntry["kind"] {
  const extension = name.split(".").pop()?.toLowerCase() ?? "";
  if (IMAGE_EXTENSIONS.has(extension)) return "image";
  if (AUDIO_EXTENSIONS.has(extension)) return "audio";
  if (VIDEO_EXTENSIONS.has(extension)) return "video";
  if (DOCUMENT_EXTENSIONS.has(extension)) return "document";
  if (extension === "pdf") return "pdf";
  if (TEXT_EXTENSIONS.has(extension)) return "text";
  return "unknown";
}

export function actionableFindingCount(
  report: ScanReport | undefined,
  preserveColorProfile: boolean,
  removeExtendedAttributes = false,
): number {
  return report ? sumFindingCounts(report.findings.filter((finding) => {
      if (finding.category === "color_profile") return !preserveColorProfile;
      if (finding.category === "macos_xattr") return removeExtendedAttributes;
      return true;
    })) : 0;
}

/** Keep aggregate counters reliable even when individually valid native counts add up beyond JS's safe integer range. */
export function sumFindingCounts(findings: Finding[]): number {
  return findings.reduce((total, finding) => {
    const count = Number.isSafeInteger(finding.count) && finding.count >= 0 ? finding.count : 0;
    if (total >= Number.MAX_SAFE_INTEGER - count) return Number.MAX_SAFE_INTEGER;
    return total + count;
  }, 0);
}

export function entryFromPath(path: string): FileEntry {
  const name = path.split(/[\\/]/).pop() || path;
  return {
    id: pathIdentity(path),
    name,
    path,
    kind: classifyFile(name),
    status: "ready",
  };
}

export function entryFromFile(file: File): FileEntry {
  const relativePath = (file as File & { webkitRelativePath?: unknown }).webkitRelativePath;
  const safeRelativePath = typeof relativePath === "string" && isBoundedText(relativePath, MAX_PATH_BYTES)
    ? relativePath
    : undefined;
  return entryFromFileParts(file.name, file.size, file.lastModified, safeRelativePath);
}

function entryFromFileParts(name: string, size: number, lastModified: number, relativePath?: string): FileEntry {
  return {
    // Browser File objects do not expose an absolute path. When a directory
    // selection or drag supplies webkitRelativePath, include it so two files
    // with the same name/size/timestamp from different folders do not collapse
    // into one queue row.
    id: `${browserPathIdentity(relativePath || name)}:${size}:${lastModified}`,
    name,
    size,
    kind: classifyFile(name),
    status: "ready",
  };
}

/** Normalize browser FileList values before they become queue state. */
export function normalizeBrowserFiles(value: unknown): FileEntry[] {
  if (!value || typeof value !== "object") return [];
  let length: number;
  try {
    const rawLength = (value as { length?: unknown }).length;
    if (typeof rawLength !== "number" || !Number.isSafeInteger(rawLength) || rawLength < 0) return [];
    // Do not silently drop the tail of a browser selection. Native picker and
    // drag/drop intake reject an oversized batch; the browser fallback must
    // preserve that same all-or-nothing contract.
    if (rawLength > MAX_BATCH_FILES) return [];
    length = rawLength;
  } catch {
    return [];
  }
  const entries: FileEntry[] = [];
  for (let index = 0; index < length; index += 1) {
    try {
      const file = (value as ArrayLike<unknown>)[index];
      if (!file || typeof file !== "object") continue;
      const candidate = file as { name?: unknown; size?: unknown; lastModified?: unknown; webkitRelativePath?: unknown };
      if (!isBoundedText(candidate.name, MAX_NAME_BYTES)
        || typeof candidate.size !== "number" || !Number.isSafeInteger(candidate.size) || candidate.size < 0
        || typeof candidate.lastModified !== "number" || !Number.isSafeInteger(candidate.lastModified) || candidate.lastModified < 0
        || (candidate.webkitRelativePath !== undefined
          && candidate.webkitRelativePath !== ""
          && !isBoundedText(candidate.webkitRelativePath, MAX_PATH_BYTES))) continue;
      entries.push(entryFromFileParts(
        candidate.name,
        candidate.size,
        candidate.lastModified,
        typeof candidate.webkitRelativePath === "string" ? candidate.webkitRelativePath : undefined,
      ));
    } catch {
      // A broken File-like object must not abort the remaining batch.
    }
  }
  return entries;
}

export interface MergeEntriesResult {
  entries: FileEntry[];
  skipped: number;
}

export function mergeEntries(current: FileEntry[], incoming: FileEntry[]): MergeEntriesResult {
  const retained = current.slice(0, MAX_BATCH_FILES);
  const known = new Set(retained.map((entry) => entry.path ? pathIdentity(entry.path) : entry.id));
  const additions = incoming.filter((entry) => {
    const identity = entry.path ? pathIdentity(entry.path) : entry.id;
    if (known.has(identity)) return false;
    known.add(identity);
    return true;
  });
  const available = Math.max(0, MAX_BATCH_FILES - retained.length);
  return {
    entries: retained.concat(additions.slice(0, available)),
    skipped: Math.max(0, additions.length - available),
  };
}

export function markEntryPaths(current: FileEntry[], paths: string[], status: FileEntry["status"]): FileEntry[] {
  const requested = new Set(paths.map(pathIdentity));
  return current.map((entry) => {
    if (!entry.path || !requested.has(pathIdentity(entry.path))) return entry;
    return status === "scanning"
      ? { ...entry, status, report: undefined, result: undefined }
      : { ...entry, status };
  });
}

export function applyScanReports(current: FileEntry[], paths: string[], reports: ScanReport[]): FileEntry[] {
  const requested = new Set(paths.map(pathIdentity));
  const byPath = new Map(reports.map((report) => [pathIdentity(report.path), report]));
  return current.map((entry) => {
    if (!entry.path || !requested.has(pathIdentity(entry.path))) return entry;
    const report = byPath.get(pathIdentity(entry.path));
    if (!report) return { ...entry, report: undefined, result: undefined, status: "ready" };
    return { ...entry, report, result: undefined, status: report.error ? "error" : "scanned" };
  });
}
