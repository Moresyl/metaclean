import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

export async function generateChecksums(directory, outputPath) {
  const outputName = path.basename(outputPath);
  const entries = (await readdir(directory, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name !== outputName && entry.name !== "SHASUMS256.txt")
    .map((entry) => entry.name)
    .sort((left, right) => left.localeCompare(right, "en"));
  if (entries.length === 0) throw new Error("No release assets were found");
  const lines = [];
  for (const name of entries) {
    if (/\r|\n/u.test(name)) throw new Error(`Unsafe release asset name: ${JSON.stringify(name)}`);
    const digest = createHash("sha256").update(await readFile(path.join(directory, name))).digest("hex");
    lines.push(`${digest}  ${name}`);
  }
  const manifest = `${lines.join("\n")}\n`;
  await writeFile(outputPath, manifest, "utf8");
  return manifest;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [directory, outputPath] = process.argv.slice(2);
  if (!directory || !outputPath) throw new Error("Usage: generate-checksums.mjs <asset-directory> <output-file>");
  const manifest = await generateChecksums(directory, outputPath);
  console.log(`Generated ${manifest.trimEnd().split("\n").length} SHA-256 entries.`);
}
