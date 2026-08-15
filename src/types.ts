export type CleanMode = "copy" | "replace";

export interface FileEntry {
  id: string;
  name: string;
  path?: string;
  size?: number;
  kind: "image" | "document" | "pdf" | "text" | "unknown";
  status: "ready" | "scanning" | "clean";
}
