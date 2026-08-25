import { describe, expect, it } from "vitest";
import type { ScanReport } from "../types";
import { actionableFindingCount, applyScanReports, classifyFile, entryFromFile, entryFromPath, markEntryPaths, mergeEntries } from "./files";

describe("classifyFile", () => {
  const groups = {
    image: [
      "jpg", "jpeg", "jpe", "png", "webp", "jxl", "gif", "bmp", "dib", "tif", "tiff",
      "heic", "heif", "heics", "heifs", "hif", "avif", "avifs",
      "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "rwl",
      "dng", "pef", "srw", "raf", "3fr", "erf", "mef", "mos", "iiq", "kdc", "dcr", "k25",
    ],
    audio: ["mp3", "wav", "flac", "aif", "aiff", "aifc", "wma", "m4a", "f4a", "f4b", "m4b", "m4p", "mka"],
    video: [
      "mp4", "mov", "m4v", "3g2", "3gp", "3gp2", "3gpp", "f4p", "f4v", "lrv", "mqv", "qt",
      "avi", "asf", "wmv", "mkv", "mks", "mk3d", "webm",
    ],
    document: ["docx", "xlsx", "pptx", "odt", "ods", "odp", "odg", "odf", "odb", "odm", "ott", "ots", "otp", "otg", "epub"],
    pdf: ["pdf"],
    text: ["txt", "md", "markdown", "html", "htm", "xhtml", "svg", "xml", "json", "csv", "tsv", "yaml", "yml", "log", "srt", "vtt"],
  } as const;
  const supportedCases = Object.entries(groups).flatMap(([kind, extensions]) =>
    extensions.map((extension) => [`sample.${extension.toUpperCase()}`, kind] as const),
  );

  /* The engine's SUPPORTED_EXTENSIONS is the same 105 entries. A row that shows
     a generic glyph for a file the engine happily cleans is the visible half of
     the two lists drifting apart. */
  it("covers every one of the engine's 105 supported extensions", () => {
    expect(supportedCases).toHaveLength(105);
    expect(new Set(supportedCases.map(([name]) => name)).size).toBe(105);
  });

  it.each(supportedCases)("classifies %s case-insensitively", (name, expected) => {
    expect(classifyFile(name)).toBe(expected);
  });

  it.each(["archive.rar", "page.psd", "sheet.numbers", "no-extension"])("rejects unsupported %s", (name) => {
    expect(classifyFile(name)).toBe("unknown");
  });
});

describe("entryFromPath", () => {
  it("supports Windows paths", () => {
    expect(entryFromPath("C:\\work\\photo.png")).toMatchObject({ name: "photo.png", kind: "image" });
  });

  it("falls back to the supplied value when no path segment exists", () => {
    expect(entryFromPath("")).toMatchObject({ name: "", path: "", kind: "unknown" });
  });
});

describe("entryFromFile", () => {
  it("uses stable file metadata for the id", () => {
    const file = new File(["hello"], "note.yaml", { lastModified: 123 });
    expect(entryFromFile(file)).toMatchObject({
      id: "note.yaml:5:123",
      name: "note.yaml",
      size: 5,
      kind: "text",
      status: "ready",
    });
  });
});

describe("mergeEntries", () => {
  it("keeps the first entry when ids repeat", () => {
    const entry = entryFromPath("C:\\photo.png");
    expect(mergeEntries([entry], [entry])).toHaveLength(1);
  });

  it("preserves order while appending only new ids", () => {
    const first = entryFromPath("first.jpg");
    const second = entryFromPath("second.mp4");
    expect(mergeEntries([first], [first, second])).toEqual([first, second]);
  });
});

describe("scan result reconciliation", () => {
  it("updates only paths that belong to the active scan", () => {
    const requested = entryFromPath("C:\\work\\first.jpg");
    const addedLater = entryFromPath("C:\\work\\later.png");
    const scanning = markEntryPaths([requested, addedLater], [requested.path!], "scanning");
    expect(scanning.map((entry) => entry.status)).toEqual(["scanning", "ready"]);

    const report: ScanReport = {
      path: requested.path!, name: requested.name, format: "JPEG", size: 10,
      supported: true, findings: [],
    };
    const completed = applyScanReports(scanning, [requested.path!], [report]);
    expect(completed[0]).toMatchObject({ status: "scanned", report });
    expect(completed[1]).toEqual(addedLater);
  });

  it("leaves a requested path retryable when the engine omits its report", () => {
    const priorReport: ScanReport = {
      path: "C:\\work\\first.jpg", name: "first.jpg", format: "JPEG", size: 10,
      supported: true, findings: [],
    };
    const entry = { ...entryFromPath(priorReport.path), status: "clean" as const, report: priorReport };
    const scanning = markEntryPaths([entry], [entry.path!], "scanning");
    expect(scanning[0]).toMatchObject({ status: "scanning", report: undefined, result: undefined });
    expect(applyScanReports(scanning, [entry.path!], [])[0]).toMatchObject({ status: "ready", report: undefined, result: undefined });
  });
});

describe("actionableFindingCount", () => {
  const report = {
    path: "photo.jpg",
    name: "photo.jpg",
    format: "JPEG",
    size: 10,
    supported: true,
    findings: [
      { category: "image_metadata", label: "EXIF", count: 2, severity: "privacy" as const },
      { category: "color_profile", label: "ICC", count: 1, severity: "informational" as const },
      { category: "macos_xattr", label: "macOS xattr", count: 2, severity: "informational" as const },
    ],
  };

  it("excludes a profile that the user chose to preserve", () => {
    expect(actionableFindingCount(report, true)).toBe(2);
  });

  it("includes a profile that the user chose to remove", () => {
    expect(actionableFindingCount(report, false)).toBe(3);
  });

  it("includes only the macOS attributes explicitly selected for removal", () => {
    expect(actionableFindingCount(report, true, true)).toBe(4);
  });

  it("handles an unscanned file", () => {
    expect(actionableFindingCount(undefined, false)).toBe(0);
  });
});
