export type CleanMode = "copy" | "replace";

export interface FileEntry {
  id: string;
  name: string;
  path?: string;
  size?: number;
  kind: "image" | "audio" | "video" | "document" | "pdf" | "text" | "unknown";
  status: "ready" | "scanning" | "scanned" | "clean" | "error";
  report?: ScanReport;
}

export interface Finding {
  category: string;
  label: string;
  count: number;
  severity: "privacy" | "provenance" | "informational";
}

export interface ScanReport {
  path: string;
  name: string;
  format: string;
  size: number;
  supported: boolean;
  findings: Finding[];
  error?: string;
}

export interface CleanResult {
  sourcePath: string;
  outputPath?: string;
  backupPath?: string;
  removed: Finding[];
  success: boolean;
  error?: string;
}

export type Page = "clean" | "history" | "privacy" | "settings";

export interface HistoryEntry {
  id: string;
  createdAt: string;
  mode: CleanMode;
  results: CleanResult[];
}

export interface ContextMenuStatus {
  available: boolean;
  enabled: boolean;
  detail: string;
}

export interface IntakeIssue {
  path: string;
  reason: string;
}

export interface IntakeResult {
  files: string[];
  skippedCount: number;
  issues: IntakeIssue[];
  limitReached: boolean;
}
