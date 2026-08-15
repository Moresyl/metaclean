import type { FileEntry } from "../types";

const IMAGE_EXTENSIONS = new Set(["jpg", "jpeg", "png", "webp", "tiff", "heic"]);
const DOCUMENT_EXTENSIONS = new Set(["docx", "xlsx", "pptx", "odt"]);
const TEXT_EXTENSIONS = new Set(["txt", "md", "html", "svg"]);

export function classifyFile(name: string): FileEntry["kind"] {
  const extension = name.split(".").pop()?.toLowerCase() ?? "";
  if (IMAGE_EXTENSIONS.has(extension)) return "image";
  if (DOCUMENT_EXTENSIONS.has(extension)) return "document";
  if (extension === "pdf") return "pdf";
  if (TEXT_EXTENSIONS.has(extension)) return "text";
  return "unknown";
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
