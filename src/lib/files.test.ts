import { describe, expect, it } from "vitest";
import { classifyFile, entryFromPath, mergeEntries } from "./files";

describe("classifyFile", () => {
  it.each([
    ["photo.JPG", "image"],
    ["report.docx", "document"],
    ["paper.pdf", "pdf"],
    ["notes.md", "text"],
    ["archive.zip", "unknown"],
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
