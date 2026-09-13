import { describe, expect, it, vi } from "vitest";
import type { ScanReport } from "../types";
import { actionableFindingCount, applyScanReports, classifyFile, entryFromFile, entryFromPath, markEntryPaths, mergeEntries, normalizeBrowserFiles, pathIdentity, sumFindingCounts } from "./files";

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
    text: ["txt", "md", "markdown", "html", "htm", "xhtml", "svg", "xml", "json", "csv", "tsv", "yaml", "yml", "log", "srt", "vtt", "css", "scss", "less", "ini", "conf", "cfg", "toml", "properties"],
  } as const;
  const supportedCases = Object.entries(groups).flatMap(([kind, extensions]) =>
    extensions.map((extension) => [`sample.${extension.toUpperCase()}`, kind] as const),
  );

  /* The engine's SUPPORTED_EXTENSIONS is the same 113 entries. A row that shows
     a generic glyph for a file the engine happily cleans is the visible half of
     the two lists drifting apart. */
  it("covers every one of the engine's 113 supported extensions", () => {
    expect(supportedCases).toHaveLength(113);
    expect(new Set(supportedCases.map(([name]) => name)).size).toBe(113);
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

  it("uses the native Windows identity for slash, case and device aliases", () => {
    expect(pathIdentity("C:/Work/Photo.PNG")).toBe("c:\\work\\photo.png");
    expect(pathIdentity("C:/Work/Ä.PNG")).toBe("c:\\work\\ä.png");
    expect(pathIdentity("\\\\?\\C:\\Work\\Photo.PNG")).toBe("c:\\work\\photo.png");
    expect(pathIdentity("\\\\?\\UNC\\Server\\Share\\Photo.PNG")).toBe("\\\\server\\share\\photo.png");
    expect(pathIdentity("C:\\Work\\sub\\.\\..\\Photo.PNG")).toBe("c:\\work\\photo.png");
    expect(pathIdentity("..\\Photo.PNG")).toBe("..\\photo.png");
    expect(pathIdentity("/Users/Alice/Photo.PNG")).toBe("/Users/Alice/Photo.PNG");
  });

  it("normalizes slash-form UNC paths only on Windows runtimes", () => {
    const originalPlatform = navigator.platform;
    vi.stubGlobal("navigator", { ...navigator, platform: "Win32" });
    expect(pathIdentity("//Server/Share/Photo.PNG")).toBe("\\\\server\\share\\photo.png");
    vi.stubGlobal("navigator", { ...navigator, platform: originalPlatform.replace(/win/iu, "Linux") });
    expect(pathIdentity("//Server/Share/Photo.PNG")).toBe("//Server/Share/Photo.PNG");
    vi.unstubAllGlobals();
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

  it("uses a browser folder path when available", () => {
    const file = new File(["hello"], "note.txt", { lastModified: 123 });
    Object.defineProperty(file, "webkitRelativePath", { value: "folder/note.txt" });
    expect(entryFromFile(file).id.replaceAll("\\", "/")).toBe("folder/note.txt:5:123");
  });

  it("bounds browser FileList intake and skips malformed File-like values", () => {
    const malformed = { name: "bad.txt", size: -1, lastModified: 1 };
    const oversized = Array.from({ length: 10_001 }, (_, index) => new File(["x"], `file-${index}.txt`, { lastModified: index }));
    expect(normalizeBrowserFiles([malformed, oversized[0]])).toHaveLength(1);
    expect(normalizeBrowserFiles(oversized)).toEqual([]);
  });

  it("keeps same-metadata files from different browser folders distinct", () => {
    const first = { name: "note.txt", size: 5, lastModified: 123, webkitRelativePath: "first/note.txt" };
    const second = { name: "note.txt", size: 5, lastModified: 123, webkitRelativePath: "second/note.txt" };
    const entries = normalizeBrowserFiles([first, second]);
    expect(entries).toHaveLength(2);
    expect(entries[0].id).not.toBe(entries[1].id);
  });

  it("deduplicates browser relative-path aliases on Windows", () => {
    const originalPlatform = navigator.platform;
    vi.stubGlobal("navigator", { ...navigator, platform: "Win32" });
    const first = normalizeBrowserFiles([{ name: "note.txt", size: 5, lastModified: 123, webkitRelativePath: "Folder/note.txt" }])[0];
    const alias = normalizeBrowserFiles([{ name: "note.txt", size: 5, lastModified: 123, webkitRelativePath: "folder\\NOTE.TXT" }])[0];
    expect(first.id).toBe(alias.id);
    vi.stubGlobal("navigator", { ...navigator, platform: originalPlatform });
    vi.unstubAllGlobals();
  });

  it("rejects an unbounded browser relative path", () => {
    expect(normalizeBrowserFiles([{ name: "note.txt", size: 5, lastModified: 123, webkitRelativePath: "界".repeat(16_385) }])).toEqual([]);
  });

  it("fails closed for broken FileList-like accessors", () => {
    const broken = { get length() { throw new Error("broken"); } };
    expect(normalizeBrowserFiles(broken)).toEqual([]);
  });
});

describe("mergeEntries", () => {
  it("keeps the first entry when ids repeat", () => {
    const entry = entryFromPath("C:\\photo.png");
    expect(mergeEntries([entry], [entry]).entries).toHaveLength(1);
  });

  it("preserves order while appending only new ids", () => {
    const first = entryFromPath("first.jpg");
    const second = entryFromPath("second.mp4");
    expect(mergeEntries([first], [first, second]).entries).toEqual([first, second]);
  });

  it("does not queue the same Windows file twice through path aliases", () => {
    const first = entryFromPath("C:\\Work\\Photo.PNG");
    const alias = entryFromPath("c:/work/photo.png");
    expect(mergeEntries([], [first, alias]).entries).toHaveLength(1);
  });

  it("caps the accumulated queue at one native batch", () => {
    const current = Array.from({ length: 10_000 }, (_, index) => entryFromPath(`current-${index}.txt`));
    const incoming = [entryFromPath("new-file.txt")];
    expect(mergeEntries(current, incoming)).toMatchObject({ skipped: 1 });
    expect(mergeEntries(current, incoming).entries).toHaveLength(10_000);
    expect(mergeEntries(current.slice(0, 9_999), incoming)).toMatchObject({ skipped: 0 });
    expect(mergeEntries(current.slice(0, 9_999), incoming).entries.at(-1)?.path).toBe("new-file.txt");
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

  it("reconciles a native report whose Windows spelling differs from the queue", () => {
    const entry = entryFromPath("C:\\Work\\photo.jpg");
    const report: ScanReport = {
      path: "c:/work/PHOTO.JPG", name: "PHOTO.JPG", format: "JPEG", size: 10,
      supported: true, findings: [],
    };
    expect(applyScanReports(markEntryPaths([entry], [entry.path!], "scanning"), [entry.path!], [report])[0]).toMatchObject({ status: "scanned", report });
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

describe("sumFindingCounts", () => {
  it("caps aggregate counts at the largest reliable JavaScript integer", () => {
    expect(sumFindingCounts([
      { category: "a", label: "a", count: Number.MAX_SAFE_INTEGER, severity: "privacy" },
      { category: "b", label: "b", count: 1, severity: "privacy" },
    ])).toBe(Number.MAX_SAFE_INTEGER);
  });

  it("ignores malformed aggregate counts instead of producing an unsafe total", () => {
    expect(sumFindingCounts([
      { category: "a", label: "a", count: -1, severity: "privacy" },
      { category: "b", label: "b", count: Number.POSITIVE_INFINITY, severity: "privacy" },
    ])).toBe(0);
  });
});
