import { describe, expect, it } from "vitest";
import { classifyFile, entryFromPath, mergeEntries } from "./files";

describe("classifyFile", () => {
  it.each([
    ["photo.JPG", "image"],
    ["animation.gif", "image"],
    ["recording.MP3", "audio"],
    ["voice.wav", "audio"],
    ["album.flac", "audio"],
    ["movie.MP4", "video"],
    ["clip.mov", "video"],
    ["report.docx", "document"],
    ["paper.pdf", "pdf"],
    ["notes.md", "text"],
    ["archive.zip", "unknown"],
    ["raw.tiff", "unknown"],
  ] as const)("classifies %s", (name, expected) => {
    expect(classifyFile(name)).toBe(expected);
  });
});

describe("entryFromPath", () => {
  it("supports Windows paths", () => {
    expect(entryFromPath("C:\\work\\photo.png")).toMatchObject({ name: "photo.png", kind: "image" });
  });
});

describe("mergeEntries", () => {
  it("keeps the first entry when ids repeat", () => {
    const entry = entryFromPath("C:\\photo.png");
    expect(mergeEntries([entry], [entry])).toHaveLength(1);
  });
});
