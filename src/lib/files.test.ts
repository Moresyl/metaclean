import { describe, expect, it } from "vitest";
import { actionableFindingCount, classifyFile, entryFromFile, entryFromPath, mergeEntries } from "./files";

describe("classifyFile", () => {
  const groups = {
    image: ["jpg", "jpeg", "jpe", "png", "webp", "gif"],
    audio: ["mp3", "wav", "flac"],
    video: ["mp4", "mov", "m4v", "m4a", "3g2", "3gp", "3gp2", "3gpp", "f4a", "f4b", "f4p", "f4v", "lrv", "m4b", "m4p", "mqv", "qt"],
    document: ["docx", "xlsx", "pptx", "odt"],
    pdf: ["pdf"],
    text: ["txt", "md", "markdown", "html", "htm", "xhtml", "svg", "xml", "json", "csv", "tsv", "yaml", "yml", "log", "srt", "vtt"],
  } as const;
  const supportedCases = Object.entries(groups).flatMap(([kind, extensions]) =>
    extensions.map((extension) => [`sample.${extension.toUpperCase()}`, kind] as const),
  );

  it("defines exactly 47 supported extensions", () => {
    expect(supportedCases).toHaveLength(47);
  });

  it.each(supportedCases)("classifies %s case-insensitively", (name, expected) => {
    expect(classifyFile(name)).toBe(expected);
  });

  it.each(["archive.zip", "raw.tiff", "no-extension"])("rejects unsupported %s", (name) => {
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
    ],
  };

  it("excludes a profile that the user chose to preserve", () => {
    expect(actionableFindingCount(report, true)).toBe(2);
  });

  it("includes a profile that the user chose to remove", () => {
    expect(actionableFindingCount(report, false)).toBe(3);
  });

  it("handles an unscanned file", () => {
    expect(actionableFindingCount(undefined, false)).toBe(0);
  });
});
