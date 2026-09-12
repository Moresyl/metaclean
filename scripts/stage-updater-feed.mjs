import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { validateUpdaterManifest } from "./generate-updater-manifest.mjs";
import { stableVersion } from "./stable-version.mjs";

export async function stageUpdaterFeed({ sourcePath, tag, repository, outputPath }) {
  const version = tag.startsWith("v") ? stableVersion(tag.slice(1)) : undefined;
  if (!version || `v${version}` !== tag) throw new Error(`Invalid release tag: ${tag}`);
  let manifest;
  try {
    manifest = JSON.parse(await readFile(sourcePath, "utf8"));
  } catch (cause) {
    throw new Error(`latest.json is not valid JSON: ${cause instanceof Error ? cause.message : String(cause)}`, { cause });
  }
  validateUpdaterManifest(manifest, { version, repository });
  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  return manifest;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [sourcePath, tag, repository, outputPath] = process.argv.slice(2);
  if (!sourcePath || !tag || !repository || !outputPath) {
    throw new Error("Usage: stage-updater-feed.mjs <latest.json> <tag> <repository> <output-file>");
  }
  const manifest = await stageUpdaterFeed({ sourcePath, tag, repository, outputPath });
  console.log(`Staged verified fallback update feed for ${manifest.version}.`);
}
