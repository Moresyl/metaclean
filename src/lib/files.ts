import type { FileEntry, ScanReport } from "../types";

/* These mirror the engine's intake list. They only choose the glyph on a queue
   row — the format itself is settled by the file's own signature during the
   scan, never by its name. */
const IMAGE_EXTENSIONS = new Set([
  "jpg", "jpeg", "jpe", "png", "webp", "gif", "bmp", "dib", "tif", "tiff",
  "heic", "heif", "heics", "heifs", "hif", "avif", "avifs",
  // Raw negatives: TIFF containers under a private magic word, plus Canon's
  // CR3, which is an ISO base media file instead.
  "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "rwl",
  "dng", "pef", "srw", "raf", "3fr", "erf", "mef", "mos", "iiq", "kdc", "dcr", "k25",
]);
const AUDIO_EXTENSIONS = new Set(["mp3", "wav", "flac", "wma", "m4a", "f4a", "f4b", "m4b", "m4p", "mka"]);
const VIDEO_EXTENSIONS = new Set([
  "mp4", "mov", "m4v", "3g2", "3gp", "3gp2", "3gpp", "f4p", "f4v", "lrv", "mqv", "qt",
  "avi", "asf", "wmv", "mkv", "mks", "mk3d", "webm",
]);
const DOCUMENT_EXTENSIONS = new Set(["docx", "xlsx", "pptx", "odt", "epub"]);
const TEXT_EXTENSIONS = new Set([
  "txt", "md", "markdown", "html", "htm", "xhtml", "svg", "xml", "json", "csv", "tsv",
  "yaml", "yml", "log", "srt", "vtt",
]);

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
  return report?.findings
    .filter((finding) => {
      if (finding.category === "color_profile") return !preserveColorProfile;
      if (finding.category === "macos_xattr") return removeExtendedAttributes;
      return true;
    })
    .reduce((total, finding) => total + finding.count, 0) ?? 0;
}

export function entryFromPath(path: string): FileEntry {
  const name = path.split(/[\\/]/).pop() || path;
  return {
    id: path,
    name,
    path,
    kind: classifyFile(name),
    status: "ready",
  };
}

export function entryFromFile(file: File): FileEntry {
  return {
    id: `${file.name}:${file.size}:${file.lastModified}`,
    name: file.name,
    size: file.size,
    kind: classifyFile(file.name),
    status: "ready",
  };
}

export function mergeEntries(current: FileEntry[], incoming: FileEntry[]): FileEntry[] {
  const known = new Set(current.map((entry) => entry.id));
  return [...current, ...incoming.filter((entry) => !known.has(entry.id))];
}
