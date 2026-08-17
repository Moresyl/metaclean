import type { FileEntry, ScanReport } from "../types";

const IMAGE_EXTENSIONS = new Set(["jpg", "jpeg", "jpe", "png", "webp", "gif"]);
const AUDIO_EXTENSIONS = new Set(["mp3", "wav", "flac"]);
const VIDEO_EXTENSIONS = new Set([
  "mp4", "mov", "m4v", "m4a", "3g2", "3gp", "3gp2", "3gpp", "f4a", "f4b", "f4p",
  "f4v", "lrv", "m4b", "m4p", "mqv", "qt",
]);
const DOCUMENT_EXTENSIONS = new Set(["docx", "xlsx", "pptx", "odt"]);
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
): number {
  return report?.findings
    .filter((finding) => finding.category !== "color_profile" || !preserveColorProfile)
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
