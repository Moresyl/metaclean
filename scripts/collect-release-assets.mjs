import { copyFile, mkdir, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory() && !/\.(?:app|appdir)$/iu.test(entry.name)) files.push(...await walk(fullPath));
    else if (entry.isFile()) files.push(fullPath);
  }
  return files;
}

export async function collectReleaseAssets(bundleDirectory, outputDirectory, expectedExtensions) {
  const extensions = expectedExtensions.map((value) => value.replace(/^\./u, ""));
  if (extensions.length === 0 || new Set(extensions).size !== extensions.length) {
    throw new Error("Expected release extensions must be unique and non-empty");
  }
  const allFiles = await walk(bundleDirectory);
  const selected = [];
  for (const extension of extensions) {
    const suffix = `.${extension}`.toLowerCase();
    const matches = allFiles.filter((file) => file.toLowerCase().endsWith(suffix));
    if (matches.length !== 1) throw new Error(`Expected exactly one .${extension} package, found ${matches.length}`);
    selected.push(matches[0]);
  }
  const names = selected.map((file) => path.basename(file));
  if (new Set(names).size !== names.length) throw new Error("Release asset basenames must be unique");
  await mkdir(outputDirectory, { recursive: true });
  for (let index = 0; index < selected.length; index += 1) {
    if ((await stat(selected[index])).size === 0) throw new Error(`Release asset is empty: ${selected[index]}`);
    await copyFile(selected[index], path.join(outputDirectory, names[index]));
  }
  return names.sort((left, right) => left.localeCompare(right, "en"));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [bundleDirectory, outputDirectory, extensions] = process.argv.slice(2);
  if (!bundleDirectory || !outputDirectory || !extensions) {
    throw new Error("Usage: collect-release-assets.mjs <bundle-directory> <output-directory> <comma-separated-extensions>");
  }
  const names = await collectReleaseAssets(bundleDirectory, outputDirectory, extensions.split(","));
  console.log(`Collected ${names.length} release assets: ${names.join(", ")}`);
}
